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
SOLANA_MARKET_PROGRAM_ID_DEVNET="" # optional after task solana-programs:deploy-devnet
SOLANA_USDC_MINT_DEVNET="4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU"
SOLANA_NETWORK_DEPLOY="devnet"
SOLANA_DEPLOY_KEYPAIR="" # defaults to ~/.config/solana/groupshop-devnet-deploy.json
SOLANA_AUTHORITY_KEYPAIR="" # defaults to ~/.config/solana/id.json
CF_PAGES_BRANCH_LANDING="main"
CF_PAGES_BRANCH_ADMIN="prod"
CF_PAGES_BRANCH_HEALTH="prod"
URL_ADMIN_PROD="https://groupshop-admin.pages.dev"
URL_HEALTH_PROD="https://groupshop-health.david-551.workers.dev"
URL_HEALTH_DASHBOARD_PROD="https://groupshop-health.pages.dev"
```

Notes:
- For `SOLANA_NETWORK=local`, taskfiles derive the market program ID and local USDC mint automatically from `.deployments/local/`.
- For `SOLANA_NETWORK=devnet`, taskfiles derive `SOLANA_MARKET_PROGRAM_ID_DEVNET` from `.deployments/devnet/market-program.json` when present, or from the explicit env var.
- `SOLANA_USDC_MINT_DEVNET` defaults to Circle's Solana Devnet USDC mint: `4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU`.
- Production deploy tasks use `SOLANA_NETWORK_DEPLOY=devnet` by default instead of `SOLANA_NETWORK`, so local development settings do not accidentally leak into Cloudflare deploys.
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

7. For non-local networks (devnet), fund the wallet with devnet SOL and devnet USDC — see below.
8. Open a product detail page.
9. Click the Phantom deposit button.
10. Approve the message signature.
11. Approve the transaction.

For `SOLANA_NETWORK=local`, deposits are built against the repo's validator at `http://localhost:9086`.

### Funding a wallet on devnet

Switch Phantom to **Devnet** (Settings → Developer Settings → Change Network).

**Devnet SOL** (transaction fees):

```
https://faucet.solana.com/
```

Paste your wallet address, select Devnet, and request SOL. If rate-limited, retry after a few minutes.

**Devnet USDC** (escrow deposits):

```
https://faucet.circle.com/
```

Select USDC on Solana, network Devnet, paste your wallet address. This also creates the USDC token account if it doesn't exist yet. The configured devnet USDC mint is:

```
4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU
```

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

### Devnet Solana setup

Create a dedicated devnet deploy keypair outside the repo:

```bash
task solana-programs:create-devnet-deploy-keypair
```

Back up this keypair securely. It is the program deploy/upgrade authority. The task suppresses seed phrase output for new keys; do not commit the JSON file or paste it into `.env`.

Fund that deploy wallet with devnet SOL, then confirm the configured deploy state:

```bash
task solana-programs:devnet-deploy-info
```

If the public devnet faucet is rate-limited, fund the printed `deploy_pubkey` manually from another devnet wallet or faucet. `task deploy-all` cannot deploy the Solana program while `deploy_sol` is `0`.

Deploy the market program to devnet:

```bash
task solana-programs:deploy-devnet
```

This writes `.deployments/devnet/market-program.json`, creates `.deployments/devnet/market-program-keypair.json` on first deploy to keep the program ID stable, and refreshes the landing manifest with the devnet program ID. The deploy keypair is only for program deployment/upgrade authority. The backend runtime authority is a separate keypair used to co-sign escrow instructions.

`task deploy-all` uploads the backend runtime secrets from local `.env` and `SOLANA_AUTHORITY_KEYPAIR` before deploying the API. To rotate the runtime authority manually:

```bash
wrangler secret put SOLANA_AUTHORITY_KEYPAIR_JSON -c cloudflare/api/wrangler.jsonc --env prod
```

Paste a Solana keypair JSON array when prompted. Do not commit this keypair or put the JSON itself in `.env`.

Fund buyer/test wallets with devnet SOL and Circle devnet USDC. The configured devnet USDC mint is:

```bash
4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU
```

Circle's docs and faucet use this mint for Solana Devnet USDC.

Default production URLs for secondary surfaces use the generated Cloudflare hostnames:
- `https://groupshop-admin.pages.dev` for admin
- `https://groupshop-health.david-551.workers.dev` for the health Worker
- `https://groupshop-health.pages.dev` for the health dashboard Pages site

Override `URL_ADMIN_PROD`, `URL_HEALTH_PROD`, and `URL_HEALTH_DASHBOARD_PROD` in `.env` after custom domains are routed in Cloudflare.

Pages deploy tasks create missing Cloudflare Pages projects automatically and deploy to explicit branch labels. `groupshop-landing` currently uses `main` as the Cloudflare Pages production branch for `groupshop.org`; admin and health default to `prod`.

Deploy all production surfaces:

```bash
task deploy-all
```

This runs:
- `task solana-programs:deploy-devnet`
- `task db:migrations-apply-prod`
- `task api:put-prod-secrets`
- `task api:deploy`
- `task landing:deploy`
- `task admin:deploy`
- `task health:deploy`
- `task health-dashboard:deploy`

Deploy only backend and landing:

```bash
task deploy
```

Or separately:

```bash
task api:deploy
task landing:deploy
task admin:deploy
task health:deploy
task health-dashboard:deploy
```

For non-local deployment you must provide real Solana values:
- `SOLANA_NETWORK_DEPLOY`
- `SOLANA_RPC_URL_DEVNET`
- `SOLANA_MARKET_PROGRAM_ID_DEVNET` or `.deployments/devnet/market-program.json`
- `SOLANA_USDC_MINT_DEVNET`
- `SOLANA_AUTHORITY_KEYPAIR` pointing at the backend runtime authority keypair file
- `URL_ADMIN_PROD`, `URL_HEALTH_PROD`, and `URL_HEALTH_DASHBOARD_PROD` if the defaults are not the deployed Cloudflare routes

For `SOLANA_NETWORK=devnet`, make sure the landing build and backend deploy use the same program ID and USDC mint values.

## Product Catalog Import

The catalog is populated from a Qogita CSV export. The import is additive (safe to re-run). To do a full refresh:

1. **Deploy** so the latest API code (including any new endpoints) is live on prod:
   ```bash
   task deploy-all
   ```

2. **Wipe** the existing catalog (products, brands, categories):
   ```bash
   task supplier:wipe-catalog-prod
   ```

3. **Import** from the Qogita CSV:
   ```bash
   task supplier:qogita-import-prod
   ```

The import applies these filters before uploading:
- Minimum MOQ of 5 (configurable via `--min-moq`)
- Maximum 10 products per category (configurable via `--max-per-category`)
- Products with a placeholder/missing image are excluded

Progress is printed every 50 categories, 100 brands, and 100 products. Expect roughly 15–20 minutes end-to-end for a full import. Errors (e.g. duplicates) are counted as skipped and do not stop the run.

The CSV file lives at the path configured by `PATH_SUPPLIER_DATA` in `.env` / `taskfiles/config.yml`. To download a fresh copy from Qogita first:

```bash
task supplier:qogita-download
```

## Useful Commands

```bash
task dev
task lint
task solana-programs:bootstrap-local
task solana-programs:write-frontend-config
task solana-programs:deploy-local
task solana-programs:deploy-devnet
task solana-programs:id-market-local
task solana-programs:id-market-devnet
task solana-tests:integration-test
task cli:market -- --help
```
