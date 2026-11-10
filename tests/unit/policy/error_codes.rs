//! Policy error codes start at 101 so they never collide with the kernel's
//! codes, which pass through settlement unchanged. This reads the policy
//! table in `specs/INTERFACES.md` and requires each code to match.

use pactlane_evaluation_policy::PolicyError;
use pactlane_interfaces::Error;

const SPEC: &str = include_str!("../../../specs/INTERFACES.md");

fn policy_table() -> std::vec::Vec<(u32, std::string::String)> {
    let start = SPEC
        .find("Policy codes start at 101")
        .expect("policy errors in spec");
    SPEC[start..]
        .lines()
        .skip_while(|line| !line.starts_with("| Code"))
        .skip(2)
        .take_while(|line| line.starts_with('|'))
        .map(|line| {
            let cells: std::vec::Vec<_> = line.split('|').map(str::trim).collect();
            (
                cells[1].parse().expect("numeric code"),
                cells[2].trim_matches('`').into(),
            )
        })
        .collect()
}

fn policy_error(name: &str) -> PolicyError {
    match name {
        "NotSigner" => PolicyError::NotSigner,
        "AlreadySigner" => PolicyError::AlreadySigner,
        "SignerConflict" => PolicyError::SignerConflict,
        "NoPendingOwner" => PolicyError::NoPendingOwner,
        other => panic!("spec lists an unknown policy error {other}"),
    }
}

#[test]
fn policy_error_codes_101_to_104_match_the_spec_table() {
    let table = policy_table();
    assert_eq!(
        table
            .iter()
            .map(|(code, _)| *code)
            .collect::<std::vec::Vec<_>>(),
        [101, 102, 103, 104]
    );
    for (code, name) in &table {
        assert_eq!(policy_error(name) as u32, *code, "{name}");
    }
}

#[test]
fn policy_codes_never_overlap_kernel_codes() {
    let highest_kernel = Error::IdOverflow as u32;
    for error in [
        PolicyError::NotSigner,
        PolicyError::AlreadySigner,
        PolicyError::SignerConflict,
        PolicyError::NoPendingOwner,
    ] {
        assert!(error as u32 > highest_kernel, "{error:?}");
    }
}
