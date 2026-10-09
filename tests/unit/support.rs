use pactlane_tests::{
    Setup, LEDGER_SECONDS, MAX_ENTRY_TTL, STARTING_BALANCE, START_SEQUENCE, START_TIME,
};
use soroban_sdk::testutils::Ledger;

#[test]
fn the_fixture_token_has_stellar_asset_decimals() {
    let s = Setup::new();
    // Budgets in tests are written in 7-decimal base units; a different
    // precision would silently scale every amount.
    assert_eq!(s.token.decimals(), 7);
}

#[test]
fn only_the_client_starts_with_funds() {
    let s = Setup::new();
    assert_eq!(s.balance(&s.client), STARTING_BALANCE);
    assert_eq!(s.balance(&s.provider), 0);
    assert_eq!(s.balance(&s.evaluator), 0);
    assert_eq!(s.balance(&s.stranger), 0);
}

#[test]
fn the_four_actors_are_distinct() {
    let s = Setup::new();
    let actors = [&s.client, &s.provider, &s.evaluator, &s.stranger];
    for (i, a) in actors.iter().enumerate() {
        for b in &actors[i + 1..] {
            assert_ne!(a, b);
        }
    }
}

#[test]
fn the_ledger_uses_testnet_archival_limits() {
    let s = Setup::new();
    let ledger = s.env.ledger().get();
    assert_eq!(ledger.timestamp, START_TIME);
    assert_eq!(ledger.sequence_number, START_SEQUENCE);
    assert_eq!(ledger.max_entry_ttl, MAX_ENTRY_TTL);
}

#[test]
fn advancing_time_also_advances_ledgers_at_the_close_rate() {
    let s = Setup::new();
    s.advance_time(60);
    assert_eq!(s.now(), START_TIME + 60);
    // TTL is counted in ledgers, so a clock that moved without the sequence
    // would make every TTL test pass vacuously.
    assert_eq!(
        s.env.ledger().sequence(),
        START_SEQUENCE + (60 / LEDGER_SECONDS) as u32
    );
}

#[test]
#[should_panic(expected = "ledger time cannot move backwards")]
fn time_cannot_be_set_backwards() {
    let s = Setup::new();
    s.set_time(START_TIME - 1);
}
