use pactlane_tests::exported_functions;

use crate::release_wasm;

#[test]
fn the_policy_exports_exactly_the_specified_functions() {
    // Every function a wallet, binding or explorer can call on the deployed
    // policy. Adding one fails this test until specs/INTERFACES.md and this
    // list change together.
    let wasm = release_wasm("pactlane-evaluation-policy");

    assert_eq!(
        exported_functions(&wasm),
        [
            "__constructor",
            "accept_owner",
            "add_signer",
            "complete",
            "is_signer",
            "kernel",
            "owner",
            "propose_owner",
            "reject",
            "remove_signer",
            "upgrade",
        ]
    );
}
