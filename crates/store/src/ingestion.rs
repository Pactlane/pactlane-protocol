//! Finalized ingestion progress, separate from executed mapping checkpoints.
use crate::checkpoint::sql_height;
use crate::postgres::validate_ident;
use crate::{Database, Result, StoreError};
use superquery_chain_api::Header;

/// Durable header ingestion without claiming that handlers have executed.
pub struct IngestionStore {
    schema: String,
}

impl IngestionStore {
    /// Select a validated project schema.
    pub fn new(schema: impl Into<String>) -> Result<Self> {
        let schema = schema.into();
        validate_ident(&schema)?;
        Ok(Self { schema })
    }

    /// Create the ingestion journal.
    pub async fn ensure_table(&self, db: &Database) -> Result<()> {
        db.batch_execute(&format!(
            "CREATE TABLE IF NOT EXISTS \"{}\".\"_superquery_ingestion_blocks\" (
             height bigint PRIMARY KEY CHECK (height >= 0), hash text NOT NULL,
             parent_hash text, timestamp timestamptz)",
            self.schema
        ))
        .await
    }

    fn latest_sql(&self) -> String {
        format!("SELECT height, hash, parent_hash, timestamp FROM \"{}\".\"_superquery_ingestion_blocks\" ORDER BY height DESC LIMIT 1", self.schema)
    }

    /// Load the last fully ingested header.
    pub async fn load(&self, db: &Database) -> Result<Option<Header>> {
        db.query(&self.latest_sql(), &[])
            .await?
            .first()
            .map(decode)
            .transpose()
    }

    /// Append one contiguous finalized header atomically.
    pub async fn commit(&self, db: &Database, header: &Header) -> Result<()> {
        let height = sql_height(header.height)?;
        if header.hash.is_empty() {
            return Err(StoreError::Decode("empty ingestion hash".into()));
        }
        let mut client = db.client().await?;
        let tx = client.transaction().await?;
        tx.batch_execute(&format!(
            "LOCK TABLE \"{}\".\"_superquery_ingestion_blocks\" IN EXCLUSIVE MODE",
            self.schema
        ))
        .await?;
        if let Some(row) = tx.query_opt(&self.latest_sql(), &[]).await? {
            let last = decode(&row)?;
            if &last == header {
                tx.commit().await?;
                return Ok(());
            }
            if !header.is_child_of(&last) {
                return Err(StoreError::MetadataConflict(
                    "ingestion requires contiguous canonical headers".into(),
                ));
            }
        }
        let sql = format!("INSERT INTO \"{}\".\"_superquery_ingestion_blocks\" (height, hash, parent_hash, timestamp) VALUES ($1, $2, $3, $4)", self.schema);
        tx.execute(
            &sql,
            &[
                &height,
                &header.hash,
                &header.parent_hash,
                &header.timestamp,
            ],
        )
        .await?;
        tx.commit().await?;
        Ok(())
    }
}

fn decode(row: &tokio_postgres::Row) -> Result<Header> {
    let height: i64 = row.try_get("height")?;
    Ok(Header {
        height: u64::try_from(height)
            .map_err(|_| StoreError::Decode("negative ingestion height".into()))?,
        hash: row.try_get("hash")?,
        parent_hash: row.try_get("parent_hash")?,
        timestamp: row.try_get("timestamp")?,
    })
}
