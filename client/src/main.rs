use crate::client::Client;

mod client;

fn main() {
    // Establish a connection to the cluster
    let mut client = Client::new();
    println!(
        "connection established, version: {}",
        client.get_version().unwrap()
    );

    // Determine who pays for fees
    client.establish_payer();

    // Check if the main program has been deployed
    client.check_program();

    // Run a circuit demo and verify on chain
    println!("start verify a proof on chain");
    client.groth16_verify();
    println!("verify success!");
}
