//! `get_jobs` pages through jobs by ID so a client can list them without one
//! call per job. Pages are bounded, so a list call can never exceed the
//! network's read limits however many jobs exist.

use pactlane_interfaces::{JobState, MAX_PAGE};
use pactlane_tests::{Setup, BUDGET};

#[test]
fn paging_from_one_lists_every_job_in_id_order() {
    let s = Setup::without_snapshots();
    let kernel = s.kernel();
    let total = 2 * MAX_PAGE as u64 + 7;
    for _ in 0..total {
        s.open_job(&kernel);
    }

    let mut listed = std::vec::Vec::new();
    let mut start = 1;
    loop {
        let page = kernel.get_jobs(&start, &MAX_PAGE);
        if page.is_empty() {
            break;
        }
        assert!(page.len() <= MAX_PAGE);
        listed.extend(page.iter().map(|job| job.id));
        start += u64::from(page.len());
    }

    assert_eq!(listed, (1..=total).collect::<std::vec::Vec<_>>());
}

#[test]
fn a_page_is_never_larger_than_max_page() {
    let s = Setup::without_snapshots();
    let kernel = s.kernel();
    for _ in 0..MAX_PAGE + 5 {
        s.open_job(&kernel);
    }

    assert_eq!(kernel.get_jobs(&1, &u32::MAX).len(), MAX_PAGE);
    assert_eq!(kernel.get_jobs(&1, &3).len(), 3);
    assert_eq!(kernel.get_jobs(&1, &0).len(), 0);
}

#[test]
fn pages_return_the_same_records_as_get_job() {
    let s = Setup::new();
    let kernel = s.kernel();
    s.open_job(&kernel);
    let funded = s.funded_job(&kernel);
    let paid = s.submitted_job(&kernel);
    kernel.complete(&paid, &s.hash(9));

    let page = kernel.get_jobs(&1, &10);

    assert_eq!(page.len(), 3);
    for job in page.iter() {
        assert_eq!(job, kernel.get_job(&job.id));
    }
    assert_eq!(page.get(1).unwrap().state, JobState::Funded);
    assert_eq!(page.get(1).unwrap().budget, BUDGET);
    assert_eq!(page.get(2).unwrap().id, paid);
    assert_eq!(funded, 2);
}

#[test]
fn edges_of_the_id_range_return_what_exists() {
    let s = Setup::new();
    let kernel = s.kernel();
    assert!(kernel.get_jobs(&1, &10).is_empty(), "no jobs yet");
    for _ in 0..3 {
        s.open_job(&kernel);
    }

    assert_eq!(kernel.get_jobs(&0, &10).len(), 3, "0 reads as 1");
    assert_eq!(kernel.get_jobs(&3, &10).len(), 1);
    assert!(kernel.get_jobs(&4, &10).is_empty());
    assert!(kernel.get_jobs(&u64::MAX, &10).is_empty());
}

#[test]
fn a_full_page_stays_well_within_testnet_limits() {
    let s = Setup::without_snapshots();
    let kernel = s.kernel();
    for _ in 0..MAX_PAGE {
        s.funded_job(&kernel);
    }

    kernel.get_jobs(&1, &MAX_PAGE);

    s.assert_well_within_tx_limits("get_jobs of a full page");
}
