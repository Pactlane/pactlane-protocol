//! HTTP RPC endpoint validation and safe display.
use url::Url;

/// Validate an endpoint without including secrets in errors.
pub fn validate_endpoint(raw: &str) -> Result<Url, String> {
    let url = Url::parse(raw).map_err(|_| "invalid RPC URL".to_string())?;
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
        return Err("RPC endpoints require http or https and a host".into());
    }
    if url.fragment().is_some() {
        return Err("RPC endpoints must not contain fragments".into());
    }
    Ok(url)
}

/// Display only the origin, omitting credentials, paths and query parameters.
pub fn redact_endpoint(raw: &str) -> String {
    Url::parse(raw)
        .map(|u| u.origin().ascii_serialization())
        .unwrap_or_else(|_| "***".into())
}
