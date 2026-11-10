//! Kernel error codes are part of the deployed ABI: clients match on the
//! number. This reads the table in `specs/INTERFACES.md` and requires every
//! code there to equal the Rust value, so the code and the spec cannot drift.

use pactlane_interfaces::Error;

const SPEC: &str = include_str!("../../../specs/INTERFACES.md");

/// `(code, name)` rows of the first `| Code | Name | ...` table after
/// `heading` in the spec.
fn error_table(heading: &str) -> std::vec::Vec<(u32, std::string::String)> {
    let start = SPEC.find(heading).expect("heading in spec");
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

fn kernel_error(name: &str) -> Error {
    match name {
        "NotFound" => Error::NotFound,
        "BadState" => Error::BadState,
        "BadActor" => Error::BadActor,
        "BadExpiry" => Error::BadExpiry,
        "Expired" => Error::Expired,
        "NotExpired" => Error::NotExpired,
        "BadBudget" => Error::BadBudget,
        "BudgetMismatch" => Error::BudgetMismatch,
        "NoProvider" => Error::NoProvider,
        "ProviderAlreadySet" => Error::ProviderAlreadySet,
        "RoleConflict" => Error::RoleConflict,
        "IdOverflow" => Error::IdOverflow,
        other => panic!("spec lists an unknown kernel error {other}"),
    }
}

#[test]
fn kernel_error_codes_match_the_spec_table() {
    let table = error_table("### Errors\n\nCodes are part of the ABI");
    assert_eq!(table.len(), 12, "the spec lists twelve kernel errors");
    for (code, name) in &table {
        assert_eq!(kernel_error(name) as u32, *code, "{name}");
    }
    // Spot-check the numbers directly, independent of the parsing.
    assert_eq!(Error::NotFound as u32, 1);
    assert_eq!(Error::BudgetMismatch as u32, 8);
    assert_eq!(Error::IdOverflow as u32, 12);
}
