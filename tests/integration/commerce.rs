use pactlane_tests::exported_functions;

use crate::release_wasm;

#[test]
fn the_kernel_exports_exactly_the_specified_functions() {
    // T17: no admin, upgrade, pause or hook function exists. Adding any
    // function to the deployed kernel fails this test until specs/INTERFACES.md
    // and this list are changed together.
    let wasm = release_wasm("pactlane-commerce");

    assert_eq!(
        exported_functions(&wasm),
        [
            "__constructor",
            "claim_refund",
            "complete",
            "create_job",
            "extend_ttl",
            "fund",
            "get_job",
            "job_count",
            "reject",
            "set_budget",
            "set_provider",
            "submit",
            "token",
        ]
    );
}
