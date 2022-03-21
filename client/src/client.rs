use std::path::Path;

use ark_ec::bn::BnParameters;
use solana_cli_config::{Config, CONFIG_FILE};
use solana_client::client_error::Result as ClientResult;
use solana_client::rpc_client::RpcClient;
use solana_client::rpc_response::RpcVersionInfo;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::instruction::AccountMeta;
use solana_sdk::native_token::LAMPORTS_PER_SOL;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::{read_keypair_file, Keypair};
use solana_sdk::signer::Signer;
use solana_sdk::transaction::Transaction;

use circuit::initialize;

const CONTRACT_SO_PATH: &str =
    "/mnt/e/Programs/zklink/groth16-sol-verifier/target/deploy/contract.so";
const CONTRACT_KEYPAIR_PATH: &str =
    "/mnt/e/Programs/zklink/groth16-sol-verifier/target/deploy/contract-keypair.json";
const SIZE: usize = 384;

pub struct Client {
    config: Config,
    connection: RpcClient,
    payer: Keypair,
    program_id: Pubkey,
}

impl Client {
    pub fn new() -> Client {
        let config = Config::load(CONFIG_FILE.as_ref().unwrap()).unwrap();
        let json_rpc_url = String::from(&config.json_rpc_url);
        println!("Get config file: {:?}", config);
        println!("Connecting to {}", config.json_rpc_url);
        Client {
            config,
            connection: RpcClient::new_with_commitment(json_rpc_url, CommitmentConfig::confirmed()),
            payer: Keypair::new(),
            program_id: read_keypair_file(CONTRACT_KEYPAIR_PATH)
                .unwrap()
                .pubkey(),
        }
    }

    fn get_payer(&self) -> Keypair {
        let keypair_path = &self.config.keypair_path;
        if self.config.keypair_path.is_empty() {
            println!(
                "Failed to create keypair from CLI config file, falling back to new random keypair"
            );
            Keypair::new()
        } else {
            read_keypair_file(&Path::new(keypair_path)).unwrap()
        }
    }

    pub fn get_version(&self) -> ClientResult<RpcVersionInfo> {
        self.connection.get_version()
    }

    pub fn establish_payer(&mut self) {
        let mut fees: u64 = 0;
        let (_, fee_calculator) = self.connection.get_recent_blockhash().unwrap();
        // Calculate the cost to fund the greeter account
        fees += self
            .connection
            .get_minimum_balance_for_rent_exemption(SIZE)
            .unwrap();
        // Calculate the cost of sending transactions
        fees += fee_calculator.lamports_per_signature * 100;

        self.payer = self.get_payer();

        let ref pub_key = self.payer.pubkey();
        let mut lamports = self.connection.get_balance(pub_key).unwrap();

        if lamports < fees {
            let sig = self.connection.request_airdrop(pub_key, fees - lamports);
            let _confirmed = self.connection.confirm_transaction(&sig.unwrap());
            lamports = self.connection.get_balance(pub_key).unwrap();
        }

        println!(
            "Using account {} containing {} SOL to pay for fees",
            pub_key,
            lamports / LAMPORTS_PER_SOL
        );
    }

    pub fn check_program(&self) {
        let program_info = self.connection.get_account(&self.program_id);
        if program_info.is_err() {
            if !Path::new(CONTRACT_SO_PATH).exists() {
                println!("Program needs to be deployed with `solana program deploy target/deploy/contract.so`");
            } else {
                println!("Program needs to be built and deployed");
            }
        } else if !program_info.unwrap().executable {
            println!("Program is not executable");
        }

        println!("Using program {}", self.program_id);
    }
    pub fn check_account(&self, seed: &str) -> Pubkey {
        // Generate the address (public key) of an account from the program so that it's easy to find later.
        let pubkey =
            Pubkey::create_with_seed(&self.payer.pubkey(), seed, &self.program_id).unwrap();

        // Check if the account has already been created
        let account = self.connection.get_account(&pubkey);
        if account.is_err() {
            println!("Creating a account {} with {} bytes", pubkey, SIZE);
            let lamports = self
                .connection
                .get_minimum_balance_for_rent_exemption(SIZE)
                .unwrap();
            let intruction = solana_sdk::system_instruction::create_account_with_seed(
                &self.payer.pubkey(),
                &pubkey,
                &self.payer.pubkey(),
                seed,
                lamports,
                SIZE as u64,
                &self.program_id,
            );
            let (recent_hash, _) = self.connection.get_recent_blockhash().unwrap();
            let transaction = Transaction::new_signed_with_payer(
                &[intruction],
                Some(&self.payer.pubkey()),
                &[&self.payer],
                recent_hash,
            );
            self.connection
                .send_and_confirm_transaction(&transaction)
                .unwrap();
        }
        pubkey
    }

    pub fn gamma_miller_loop(&self, key: Pubkey, prepared_input: Vec<u8>) {
        let keys = vec![key];
        let mut j: u8 = 0;
        for i in (1..ark_bn254::Parameters::ATE_LOOP_COUNT.len()).rev() {
            let mut data = vec![0, i as u8, j];
            data.extend(prepared_input.iter());
            self.send_transction(&keys, data);
            j += 1;
            if ark_bn254::Parameters::ATE_LOOP_COUNT[i - 1] == 1
                || ark_bn254::Parameters::ATE_LOOP_COUNT[i - 1] == -1
            {
                j += 1;
            }
        }

        let mut data = vec![0, 0, j];
        data.extend(prepared_input.iter());
        self.send_transction(&keys, data);
    }

    pub fn delta_miller_loop(&self, key: Pubkey, proof_c: Vec<u8>) {
        let keys = vec![key];
        let mut j: u8 = 0;
        for i in (1..ark_bn254::Parameters::ATE_LOOP_COUNT.len()).rev() {
            let mut data = vec![1, i as u8, j];
            data.extend(proof_c.iter());
            self.send_transction(&keys, data);
            j += 1;
            if ark_bn254::Parameters::ATE_LOOP_COUNT[i - 1] == 1
                || ark_bn254::Parameters::ATE_LOOP_COUNT[i - 1] == -1
            {
                j += 1;
            }
        }

        let mut data = vec![1, 0, j];
        data.extend(proof_c.iter());
        self.send_transction(&keys, data);
    }

    pub fn final_exponentiation(&self, keys: &Vec<Pubkey>, qap: Vec<u8>) {
        let gamma_key = keys[0];
        let delta_key = keys[1];
        let final_key = keys[2];
        // first, create account for y0..y16
        let mut final_keys = vec![];
        for i in 0..17 {
            final_keys.push(self.check_account(i.to_string().as_str()));
        }

        // prepare_final_data
        let mut data = vec![2, 0, 0];
        data.extend(qap.iter());
        let k = vec![gamma_key, delta_key, final_key];
        self.send_transction(&k, data);

        // easy_part1
        let data = vec![3, 0, 0];
        let k = vec![final_key];
        self.send_transction(&k, data);

        // easy_part2
        let data = vec![4, 0, 0];
        let k = vec![final_key];
        self.send_transction(&k, data);

        // hard_part_y0
        for i in 0..63 {
            let data = vec![5, 0, i];
            let k = vec![final_key, final_keys[0]];
            self.send_transction(&k, data);
        }

        // hard_part_y1
        let data = vec![6, 0, 64];
        let k = vec![final_keys[0], final_keys[1]];
        self.send_transction(&k, data);

        // hard_part_y3
        let data = vec![7, 0, 0];
        let k = vec![final_keys[0], final_keys[3]];
        self.send_transction(&k, data);

        // hard_part_y4
        for i in 0..63 {
            let data = vec![8, 0, i];
            let k = vec![final_keys[3], final_keys[4]];
            self.send_transction(&k, data);
        }

        // hard_part_y6
        for i in 0..63 {
            let data = vec![9, 0, i];
            let k = vec![final_keys[4], final_keys[6]];
            self.send_transction(&k, data);
        }

        // hard_part_y8
        let data = vec![10, 0, 0];
        let k = vec![final_keys[3], final_keys[4], final_keys[6], final_keys[8]];
        self.send_transction(&k, data);

        // hard_part_y9
        let data = vec![11, 0, 0];
        let k = vec![final_keys[1], final_keys[8], final_keys[9]];
        self.send_transction(&k, data);

        // hard_part_y11
        let data = vec![12, 0, 0];
        let k = vec![final_keys[4], final_keys[8], final_key, final_keys[11]];
        self.send_transction(&k, data);

        // hard_part_y13
        let data = vec![13, 0, 0];
        let k = vec![final_keys[9], final_keys[11], final_keys[13]];
        self.send_transction(&k, data);

        // hard_part_y14
        let data = vec![14, 0, 0];
        let k = vec![final_keys[8], final_keys[13], final_keys[14]];
        self.send_transction(&k, data);

        // hard_part_y15
        let data = vec![15, 0, 0];
        let k = vec![final_keys[9], final_key, final_keys[15]];
        self.send_transction(&k, data);

        // hard_part_y16
        let data = vec![16, 0, 0];
        let k = vec![final_keys[14], final_keys[15]];
        self.send_transction(&k, data);
    }

    pub fn groth16_verify(&self) {
        // run a circuit demo
        let (proof_c, prepared_input, qap) = initialize().unwrap();
        println!("run a circuit demo, get input and proof");

        // create accounts for verify
        let mut keys = vec![];
        keys.push(self.check_account("gamma"));
        keys.push(self.check_account("delta"));
        keys.push(self.check_account("final"));

        // gamma miller loop
        println!("running gamma miller loop");
        self.gamma_miller_loop(keys[0], prepared_input);

        // delta miller loop
        println!("running delta miller loop");
        self.delta_miller_loop(keys[1], proof_c);

        // final exponentiation
        println!("running final exponentiation");
        self.final_exponentiation(&keys, qap);
    }

    pub fn send_transction(&self, keys: &Vec<Pubkey>, data: Vec<u8>) {
        let accounts = keys
            .iter()
            .map(|key| AccountMeta::new(*key, false))
            .collect();
        let (recent_hash, _) = self.connection.get_recent_blockhash().unwrap();

        let i1 = solana_sdk::compute_budget::request_units(1_000_000 as u32);

        let i2 = solana_sdk::instruction::Instruction::new_with_bytes(
            self.program_id,
            data.as_slice(),
            accounts,
        );
        let transaction = Transaction::new_signed_with_payer(
            &[i1, i2],
            Some(&self.payer.pubkey()),
            &[&self.payer],
            recent_hash,
        );
        self.connection
            .send_and_confirm_transaction(&transaction)
            .unwrap();
    }
}
