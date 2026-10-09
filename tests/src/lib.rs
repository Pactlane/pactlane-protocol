//! Shared fixtures for Pactlane contract tests.
//!
//! Every test starts from the same ledger: a fixed time, testnet's real
//! state-archival limits, and a 7-decimal Stellar Asset Contract standing in
//! for USDC. Fixed values keep test snapshots stable across runs.

use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token::{StellarAssetClient, TokenClient},
    Address, BytesN, Env,
};

/// Ledger time at the start of every test: 2027-01-15T08:00:00Z.
pub const START_TIME: u64 = 1_800_000_000;

/// Ledger sequence at the start of every test.
pub const START_SEQUENCE: u32 = 1_000;

/// Testnet state-archival settings, read with `stellar network settings` on
/// 2026-10-08. TTL tests are only meaningful against the real limits.
pub const MAX_ENTRY_TTL: u32 = 3_110_400;
pub const MIN_PERSISTENT_ENTRY_TTL: u32 = 120_960;
pub const MIN_TEMP_ENTRY_TTL: u32 = 720;

/// Target ledger close time on testnet, in seconds.
pub const LEDGER_SECONDS: u64 = 5;

/// One whole token in base units. Stellar assets have 7 decimals.
pub const USDC: i128 = 10_000_000;

/// What the client holds when a test starts.
pub const STARTING_BALANCE: i128 = 1_000 * USDC;

/// A test environment with a token and four distinct, unrelated accounts.
pub struct Setup {
    pub env: Env,
    pub token: TokenClient<'static>,
    pub token_admin: StellarAssetClient<'static>,
    /// Creates and funds jobs. Starts with [`STARTING_BALANCE`].
    pub client: Address,
    /// Does the work. Starts with nothing.
    pub provider: Address,
    /// Completes or rejects. Starts with nothing.
    pub evaluator: Address,
    /// Holds no role in any job; used to prove outsiders are refused.
    pub stranger: Address,
}

impl Setup {
    /// A fresh environment with every authorization mocked.
    ///
    /// Mocking lets tests drive any call; tests about authorization assert on
    /// `env.auths()` to check exactly who was required to sign.
    pub fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().with_mut(|ledger| {
            ledger.timestamp = START_TIME;
            ledger.sequence_number = START_SEQUENCE;
            ledger.max_entry_ttl = MAX_ENTRY_TTL;
            ledger.min_persistent_entry_ttl = MIN_PERSISTENT_ENTRY_TTL;
            ledger.min_temp_entry_ttl = MIN_TEMP_ENTRY_TTL;
        });

        let issuer = Address::generate(&env);
        let asset = env.register_stellar_asset_contract_v2(issuer);
        let token = TokenClient::new(&env, &asset.address());
        let token_admin = StellarAssetClient::new(&env, &asset.address());

        let client = Address::generate(&env);
        let provider = Address::generate(&env);
        let evaluator = Address::generate(&env);
        let stranger = Address::generate(&env);
        token_admin.mint(&client, &STARTING_BALANCE);

        Self {
            env,
            token,
            token_admin,
            client,
            provider,
            evaluator,
            stranger,
        }
    }

    /// A distinct 32-byte hash for commitments such as `spec_hash`.
    pub fn hash(&self, seed: u8) -> BytesN<32> {
        BytesN::from_array(&self.env, &[seed; 32])
    }

    /// The current ledger timestamp.
    pub fn now(&self) -> u64 {
        self.env.ledger().timestamp()
    }

    /// Moves ledger time forward, advancing the sequence at testnet's close
    /// rate so time and TTL (counted in ledgers) stay consistent.
    pub fn advance_time(&self, seconds: u64) {
        let ledgers = u32::try_from(seconds / LEDGER_SECONDS).expect("advance fits in u32 ledgers");
        self.env.ledger().with_mut(|ledger| {
            ledger.timestamp += seconds;
            ledger.sequence_number += ledgers;
        });
    }

    /// Moves ledger time to exactly `timestamp`, which must not be in the past.
    pub fn set_time(&self, timestamp: u64) {
        let now = self.now();
        assert!(timestamp >= now, "ledger time cannot move backwards");
        self.advance_time(timestamp - now);
    }

    /// Token balance of `who`.
    pub fn balance(&self, who: &Address) -> i128 {
        self.token.balance(who)
    }
}

impl Default for Setup {
    fn default() -> Self {
        Self::new()
    }
}
