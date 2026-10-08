use hello_world::HelloWorldClient;
use soroban_sdk::{vec, Env, String};

use crate::release_wasm;

#[test]
fn the_deployed_wasm_greets_the_given_name() {
    let env = Env::default();
    let wasm = release_wasm("hello-world");
    // Registering the bytes runs the contract in the Wasm VM, so this catches
    // anything that only breaks once compiled for `wasm32v1-none`.
    let contract_id = env.register(wasm.as_slice(), ());
    let client = HelloWorldClient::new(&env, &contract_id);

    let words = client.hello(&String::from_str(&env, "Pactlane"));

    assert_eq!(
        words,
        vec![
            &env,
            String::from_str(&env, "Hello"),
            String::from_str(&env, "Pactlane"),
        ]
    );
}
