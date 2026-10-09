//! Shared fixtures for Pactlane contract tests.
//!
//! Every test starts from the same ledger: a fixed time, testnet's real
//! state-archival limits, and a 7-decimal Stellar Asset Contract standing in
//! for USDC. Fixed values keep test snapshots stable across runs.

use pactlane_commerce::{CommerceKernel, CommerceKernelClient};
use soroban_sdk::{
    testutils::{
        Address as _, AuthorizedFunction, ContractEvents, EnvTestConfig, Events, Ledger,
        StellarAssetIssuer,
    },
    token::{StellarAssetClient, TokenClient},
    Address, BytesN, Env, Symbol,
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

/// How long jobs opened by [`Setup::open_job`] run: one week.
pub const JOB_DURATION: u64 = 7 * 24 * 60 * 60;

/// Budget of jobs funded by [`Setup::funded_job`].
pub const BUDGET: i128 = 4 * USDC;

/// A test environment with a token and four distinct, unrelated accounts.
pub struct Setup {
    pub env: Env,
    pub token: TokenClient<'static>,
    pub token_admin: StellarAssetClient<'static>,
    /// The token's classic issuer, for issuer flags such as revocability.
    pub issuer: StellarAssetIssuer,
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
        Self::with_env(Env::default())
    }

    /// Like [`Setup::new`], but writes no test snapshot. For property tests,
    /// which build hundreds of environments per test.
    pub fn without_snapshots() -> Self {
        Self::with_env(Env::new_with_config(EnvTestConfig {
            capture_snapshot_at_drop: false,
        }))
    }

    fn with_env(env: Env) -> Self {
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
        let issuer = asset.issuer();

        let client = Address::generate(&env);
        let provider = Address::generate(&env);
        let evaluator = Address::generate(&env);
        let stranger = Address::generate(&env);
        token_admin.mint(&client, &STARTING_BALANCE);

        Self {
            env,
            token,
            token_admin,
            issuer,
            client,
            provider,
            evaluator,
            stranger,
        }
    }

    /// Deploys a fresh kernel bound to the fixture token.
    pub fn kernel(&self) -> CommerceKernelClient<'static> {
        let id = self
            .env
            .register(CommerceKernel, (self.token.address.clone(),));
        CommerceKernelClient::new(&self.env, &id)
    }

    /// Opens a job between the fixture's client, provider and evaluator that
    /// expires [`JOB_DURATION`] from now. Returns its ID.
    pub fn open_job(&self, kernel: &CommerceKernelClient) -> u64 {
        kernel.create_job(
            &self.client,
            &Some(self.provider.clone()),
            &self.evaluator,
            &(self.now() + JOB_DURATION),
            &self.hash(1),
        )
    }

    /// Opens a job, agrees a budget of [`BUDGET`], and funds it.
    pub fn funded_job(&self, kernel: &CommerceKernelClient) -> u64 {
        let id = self.open_job(kernel);
        kernel.set_budget(&id, &self.client, &BUDGET);
        kernel.fund(&id, &BUDGET);
        id
    }

    /// A funded job whose provider has submitted work hash `hash(2)`.
    pub fn submitted_job(&self, kernel: &CommerceKernelClient) -> u64 {
        let id = self.funded_job(kernel);
        kernel.submit(&id, &self.hash(2));
        id
    }

    /// Like [`Setup::open_job`], but with no provider assigned yet.
    pub fn open_job_without_provider(&self, kernel: &CommerceKernelClient) -> u64 {
        kernel.create_job(
            &self.client,
            &None,
            &self.evaluator,
            &(self.now() + JOB_DURATION),
            &self.hash(1),
        )
    }

    /// Asserts that the last call required exactly one signature: `signer`'s,
    /// for `function` on `contract`.
    pub fn assert_authorized_by(&self, signer: &Address, contract: &Address, function: &str) {
        let auths = self.env.auths();
        assert_eq!(
            auths.len(),
            1,
            "expected exactly one authorizer, got {auths:?}"
        );
        let (who, invocation) = &auths[0];
        assert_eq!(who, signer, "wrong authorizer");
        match &invocation.function {
            AuthorizedFunction::Contract((address, name, _)) => {
                assert_eq!(address, contract, "authorized the wrong contract");
                assert_eq!(
                    name,
                    &Symbol::new(&self.env, function),
                    "authorized the wrong function"
                );
            }
            other => panic!("expected a contract call, got {other:?}"),
        }
    }

    /// Events `contract` emitted during the last call.
    ///
    /// Only the most recent invocation counts, so check events before any
    /// read such as `get_job`, which would replace them with nothing.
    pub fn events_of(&self, contract: &Address) -> ContractEvents {
        self.env.events().all().filter_by_contract(contract)
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

/// Names of the functions a compiled contract exports, sorted, read from the
/// `contractspecv0` custom section of its Wasm. This is what wallets, bindings
/// and explorers see, so it is the surface to assert on.
pub fn exported_functions(wasm: &[u8]) -> std::vec::Vec<std::string::String> {
    use soroban_sdk::xdr::{Limited, Limits, ReadXdr, ScSpecEntry};

    let spec = custom_section(wasm, "contractspecv0").expect("Wasm has a contract spec");
    let mut names: std::vec::Vec<_> = ScSpecEntry::read_xdr_iter(&mut Limited::new(
        std::io::Cursor::new(spec),
        Limits::none(),
    ))
    .filter_map(|entry| match entry.expect("valid spec entry") {
        ScSpecEntry::FunctionV0(function) => Some(function.name.to_utf8_string_lossy()),
        _ => None,
    })
    .collect();
    names.sort();
    names
}

/// The payload of the Wasm custom section called `name`.
fn custom_section<'a>(wasm: &'a [u8], name: &str) -> Option<&'a [u8]> {
    assert_eq!(&wasm[..4], b"\0asm", "not a Wasm module");
    let mut rest = &wasm[8..]; // magic + version
    while !rest.is_empty() {
        let id = rest[0];
        let (size, read) = leb128_u32(&rest[1..]);
        let start = 1 + read;
        let body = &rest[start..start + size as usize];
        rest = &rest[start + size as usize..];
        if id == 0 {
            let (len, read) = leb128_u32(body);
            if &body[read..read + len as usize] == name.as_bytes() {
                return Some(&body[read + len as usize..]);
            }
        }
    }
    None
}

/// Decodes an unsigned LEB128 value; returns it and the bytes consumed.
fn leb128_u32(bytes: &[u8]) -> (u32, usize) {
    let mut value = 0u32;
    for (i, byte) in bytes.iter().enumerate().take(5) {
        value |= u32::from(byte & 0x7f) << (7 * i);
        if byte & 0x80 == 0 {
            return (value, i + 1);
        }
    }
    panic!("malformed LEB128 in Wasm section header")
}
