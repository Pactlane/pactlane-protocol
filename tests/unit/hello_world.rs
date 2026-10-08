use hello_world::{HelloWorld, HelloWorldClient};
use soroban_sdk::{vec, Env, String};

#[test]
fn hello_greets_the_given_name() {
    let env = Env::default();
    let contract_id = env.register(HelloWorld, ());
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
