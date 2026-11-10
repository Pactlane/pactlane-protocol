//! The TTL guarantee in `specs/INTERFACES.md`: jobs last at most half the
//! network's maximum entry TTL, so a live job written once cannot archive
//! before it expires. Checked in ledgers, the unit TTL is counted in.

use pactlane_interfaces::MAX_JOB_DURATION;
use pactlane_tests::{LEDGER_SECONDS, MAX_ENTRY_TTL};

fn ledgers(seconds: u64, close_seconds: u64) -> u64 {
    seconds.div_ceil(close_seconds)
}

#[test]
fn the_longest_job_lasts_at_most_half_the_maximum_entry_ttl() {
    let job = ledgers(MAX_JOB_DURATION, LEDGER_SECONDS);

    // Exactly half at testnet's 5-second close: 2x headroom, no more.
    assert!(
        2 * job <= u64::from(MAX_ENTRY_TTL),
        "{job} ledgers is more than half of {MAX_ENTRY_TTL}"
    );
    assert_eq!(job, 1_555_200);
}

#[test]
fn the_longest_job_outlives_no_entry_even_if_ledgers_close_faster() {
    // Ledgers that close faster than the 5-second target eat the headroom;
    // the guarantee only breaks below a 2.5-second average close.
    for close_seconds in [3, 4, 5, 6] {
        let job = ledgers(MAX_JOB_DURATION, close_seconds);
        assert!(
            job < u64::from(MAX_ENTRY_TTL),
            "at {close_seconds}s per ledger a job spans {job} ledgers"
        );
    }
}
