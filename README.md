# Groth16 verifier on Solana

This project is an Implementation of the Groth16 zk-SNARK proving system on Solana.

The project comprises of:

- An on-chain proof verifier program
- A circuit demo
- A client can send proof and input to verifier program

## Quick Start

The following dependencies are required to build and run this example, depending on your OS, they may already be installed:

- Install Rust v1.56.1 or later from https://rustup.rs/

- Install Solana v1.8.1 or later from https://docs.solana.com/cli/install-solana-cli-tools

### Configure CLI

> you're on Windows, it is recommended to use [WSL](https://docs.microsoft.com/en-us/windows/wsl/install-win10) to run these commands

1. Set CLI config url to localhost cluster

```
solana config set --url localhost
```

2. Create CLI Keypair

If this is your first time using the Solana CLI, you will need to generate a new keypair:

```
solana-keygen new
```

### Start local Solana cluster

This example connects to a local Solana cluster by default.

Start a local Solana cluster:

```
solana-test-validator
```

> **Note**: You may need to do some [system tuning](https://docs.solana.com/running-validator/validator-start#system-tuning) (and restart your computer) to get the validator to run

Listen to transaction logs:

```
solana logs
```

### Build the on-chain program

```
cd contract
cargo build-bpf
```

### Deploy the on-chain program

```
solana program deploy target/deploy/contract.so
```

### Build and run the client

```
cd client
cargo build
../target/debug/client
```

