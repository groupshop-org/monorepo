# Groupshop

See [COPILOT Vetting](./docs/COPILOT-VETTING.md) for the Colosseum Copilot Vetting results.

## Status

Work in progress for the Colosseum Hackathon.

## Overview

Groupshop is a Rust/WASM + Cloudflare Workers app with a Solana escrow program.

Main components:
- `packages/frontend/landing`: customer-facing landing site and Phantom deposit flow
- `packages/frontend/admin`: admin UI
- `packages/backend/api`: Cloudflare Worker API
- `packages/solana-programs/market`: Solana escrow program
- `packages/cli`: native Rust CLI for market-program operations

## Prerequisites

Install these before trying to build or run anything:
- Rust toolchain
- `task`
- Node.js + npm
- `trunk`
- `worker-build`
- `wasm-bindgen-cli`
- Cloudflare `wrangler`
- Solana CLI tools: `solana`, `solana-keygen`, `solana-test-validator`, `cargo-build-sbf`
- SPL Token CLI: `spl-token`

Repo-specific install commands:

```bash
npm install
cargo install worker-build
cargo install wasm-bindgen-cli
```

The Solana CLI path used by this repo is expected to be on `PATH`:

```bash
~/.local/share/solana/install/active_release/bin
```

## Environment

Copy the example env file:

```bash
cp .env.example .env
```

Minimum required values for local development:
- `API_TOKEN_SIGNING_KEY`
- `API_OPENID_GOOGLE_CLIENT_ID`
- `API_OPENID_GOOGLE_CLIENT_SECRET`
- `SOLANA_NETWORK`

Generate a token signing key:

```bash
openssl rand -hex 32
```

Relevant Solana env vars:

```bash
SOLANA_NETWORK="local"   # local or devnet
SOLANA_RPC_URL_DEVNET="https://api.devnet.solana.com"
SOLANA_MARKET_PROGRAM_ID_DEVNET=""
SOLANA_USDC_MINT_DEVNET=""
```

Notes:
- For `SOLANA_NETWORK=local`, taskfiles derive the market program ID and local USDC mint automatically from `.deployments/local/`.
- For `SOLANA_NETWORK=devnet`, you must supply `SOLANA_MARKET_PROGRAM_ID_DEVNET` and `SOLANA_USDC_MINT_DEVNET`.
- The backend authority signer used for local development defaults to `~/.config/solana/id.json` and is passed into the Worker as `SOLANA_AUTHORITY_KEYPAIR_JSON`.

## Local Development

Bring up the full local stack:

```bash
task dev
```

This starts:
- local Solana validator
- local Solana program build watcher
- local Solana bootstrap for the landing/API flow
- backend API Worker
- landing frontend
- admin frontend
- health services

Keep that terminal open. Use a second terminal for all other commands.

### What local Solana bootstrap does

The dev flow now bootstraps the Solana side automatically:
- waits for the local validator
- builds and deploys the `market` program
- creates a reusable local USDC mint if needed
- writes `.deployments/local/market-program.json`
- writes `.deployments/local/usdc-mint.json`
- writes the frontend manifest consumed by the landing app:
  [packages/frontend/landing/assets/solana-deployments.json](./packages/frontend/landing/assets/solana-deployments.json)

You can run that bootstrap directly:

```bash
task solana-programs:bootstrap-local
```

If you only want to refresh the frontend Solana manifest:

```bash
task solana-programs:write-frontend-config
```

## Phantom Wallet Deposit Flow

The landing app uses Phantom for escrow deposits.

Current flow:
1. User signs in to Groupshop.
2. User opens a product page.
3. Landing app connects to Phantom.
4. Landing app requests a backend deposit intent.
5. Phantom signs a wallet-ownership message.
6. Backend verifies the message and returns a co-signed deposit payload.
7. Phantom signs and sends the escrow deposit transaction.

The backend authority co-signs deposit transactions server-side. For local development this uses your local Solana keypair JSON. For remote deployment you must provide the authority key via Worker secret/config.

Current first-pass limitation:
- `shipping_amount` is still `0` in the backend quote path. The escrow path is wired and working at the infrastructure level, but shipping pricing still needs product/business logic.

## Building

### Install frontend wallet bridge dependencies

The Phantom bridge is bundled from:
- `packages/frontend/landing/js/phantom_bridge.entry.js`

Build it manually with:

```bash
npm run build:landing-wallet
```

This is also run automatically by the landing taskfile.

### Build the landing app

Development:

```bash
task landing:dev
```

Production build:

```bash
task landing:build
```

### Build the backend API

Development build once:

```bash
task api:build-dev-once
```

Production build:

```bash
task api:deploy
```

### Build the Solana program

Development build:

```bash
task solana-programs:build-dev
```

Release build:

```bash
task solana-programs:build-release
```

## Testing And Verification

Primary repo verification:

```bash
task lint
```

This runs:
- `cargo fmt --all -- --check`
- WASM checks for the frontend/backend packages
- native check for the CLI
- Solana program build

### Solana program integration test

The repo includes a local validator integration test flow:

```bash
task solana-tests:integration-test
```

This exercises:
- local validator reachability
- program deployment
- local USDC mint/account setup
- pool initialization
- deposit
- lock/release
- refund-mode and refund claim flow

### Manual Phantom deposit test

Recommended local manual test:

1. Run `task dev`.
2. Open the landing app at `http://127.0.0.1:9082`.
3. Register or sign in.
4. Install Phantom in the browser if it is not already installed.
5. For `SOLANA_NETWORK=local`, add Phantom's custom RPC endpoint `http://127.0.0.1:9086` and switch to it. Otherwise switch Phantom to the network matching `SOLANA_NETWORK`.
6. Set `SOLANA_WALLET_DEV` in `.env` to your Phantom wallet address. Then `task dev` will automatically ensure that wallet has local SOL and test USDC on startup.

To fund or top up the same wallet manually:

```bash
task solana-programs:fund-local-wallet -- "$SOLANA_WALLET_DEV"
```

7. For non-local networks, ensure the wallet has SOL and USDC on the configured cluster.
8. Open a product detail page.
9. Click the Phantom deposit button.
10. Approve the message signature.
11. Approve the transaction.

For `SOLANA_NETWORK=local`, deposits are built against the repo's validator at `http://localhost:9086`.

## Solana CLI Helpers

Show the deployed local market program ID:

```bash
task solana-programs:id-market-local
```

Run a market CLI command:

```bash
task cli:market -- --help
```

Examples:

```bash
task cli:market -- pdas --program-id "$(task solana-programs:id-market-local)" --product-slug my-product
task cli:market -- lock-pool --program-id "$(task solana-programs:id-market-local)" --product-slug my-product
```

## Deployment

Deploy backend and landing:

```bash
task deploy
```

Or separately:

```bash
task api:deploy
task landing:deploy
```

For non-local deployment you must provide real Solana values:
- `SOLANA_NETWORK`
- `SOLANA_RPC_URL`
- `SOLANA_MARKET_PROGRAM_ID`
- `SOLANA_USDC_MINT`
- `SOLANA_AUTHORITY_KEYPAIR_JSON`

For `SOLANA_NETWORK=devnet`, make sure the landing build and backend deploy use the same program ID and USDC mint values.

## Useful Commands

```bash
task dev
task lint
task solana-programs:bootstrap-local
task solana-programs:write-frontend-config
task solana-programs:deploy-local
task solana-programs:id-market-local
task solana-tests:integration-test
task cli:market -- --help
```
