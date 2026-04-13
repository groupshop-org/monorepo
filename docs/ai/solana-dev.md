# Solana Development

Local Solana development in this repo uses `solana-test-validator` and a Rust CLI to interact with deployed programs.

## Architecture

| Component | Path | Role |
|---|---|---|
| On-chain programs | `packages/solana-programs/` | BPF/SBF programs built with `pinocchio` via `cargo build-sbf` |
| CLI | `packages/cli/` | Native Rust binary for sending transactions to programs |
| Validator taskfile | `taskfiles/solana/validator.yml` | Runs `solana-test-validator` on the dev RPC port |
| Programs taskfile | `taskfiles/solana/programs.yml` | Builds and deploys programs to the local validator |
| CLI taskfile | `taskfiles/cli/cli.yml` | Runs CLI commands with repo env vars |
| Debug taskfile | `taskfiles/debug/debug.yml` | Scratch task for quick deploy-and-invoke cycles |

## Solana Toolchain

The Solana CLI tools (`solana`, `solana-keygen`, `cargo-build-sbf`, `solana-test-validator`) are installed at `~/.local/share/solana/install/active_release/bin/`. This path must be on `$PATH` for the task commands to work.

## Running Locally

1. Start the dev environment: `task dev` (this starts the validator alongside other services)
2. Build and deploy a program: `task solana-programs:build-dev && task solana-programs:deploy-local`
3. Invoke the deployed program: `task cli:invoke -- <PROGRAM_ID>`

Or use the scratch task which does all three: `task debug:scratch`

## How Program Invocation Works

There is no `solana invoke` CLI command in the Solana toolchain. To execute a deployed program, you must build and send a transaction containing an instruction that targets the program. The `groupshop-cli` binary handles this:

1. Creates a temporary keypair
2. Airdrops SOL to fund the transaction fee
3. Builds a transaction with an instruction targeting the program ID
4. Simulates the transaction to capture program logs
5. Sends and confirms the transaction on-chain

The CLI uses `solana-sdk` and `solana-client` crates (native Rust, not WASM).

## CLI Commands

```sh
# Invoke a program by ID (RPC URL injected from config.yml)
task cli:invoke -- <PROGRAM_ID>

# Show CLI help
task cli:help
```

## RPC URL

The `--rpc-url` flag is **required** by the CLI binary — it has no default. The RPC port is defined once in `taskfiles/config.yml` as `PORT_SOLANA_VALIDTOR_DEV_RPC`, which feeds `URL_SOLANA_VALIDATOR_RPC_DEV`. All taskfile targets that call the CLI must pass `--rpc-url` from that variable. Never hardcode the port or URL in Rust code, taskfiles, or docs.

## Adding a New Program

1. Create a new crate under `packages/solana-programs/` with `crate-type = ["cdylib"]`
2. Use `pinocchio` for the entrypoint and `solana-program-log` for logging
3. Add the build and deploy targets to `taskfiles/solana/programs.yml`
4. Build with `cargo build-sbf` (not regular `cargo build`)
5. Deploy to local validator with `solana program deploy --url <RPC_URL> <path-to-.so>`

## Constraints

- On-chain programs compile to BPF/SBF via `cargo build-sbf`, not to `wasm32-unknown-unknown`
- The CLI compiles as a native binary — it is NOT a WASM target
- `solana-sdk` and `solana-client` crates are heavy; only the CLI depends on them
- On-chain programs use `pinocchio` (lightweight) instead of `solana-program` (heavy)
- Never add `solana-sdk` or `solana-client` as dependencies of on-chain programs
