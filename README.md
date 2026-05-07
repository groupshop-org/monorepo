# Groupshop

**Real-world group buying, coordinated on Solana.**

Buyers pool deposits into an on-chain escrow until a group hits its threshold. If the deal fills, the order goes to the supplier. If it doesn't, every buyer gets their money back — no middleman holding the float.

- 🛒 Live site: <https://groupshop.org>
- 🩺 Health dashboard: <https://health.groupshop.org>
- 🏆 Built for the [Colosseum Frontier Hackathon](https://www.colosseum.com/hackathon)

> **Beta — Solana devnet only.** No real money or fulfillment yet. See [the privacy policy](https://groupshop.org/privacy) for the full beta disclaimer.

## Architecture

A single Rust workspace targeting WASM on the edge, with a native Solana program at the core.

| Package | Role |
| --- | --- |
| `packages/frontend/landing` | Customer-facing landing site + Phantom deposit flow (Yew → WASM) |
| `packages/frontend/admin` | Admin UI (Yew → WASM) |
| `packages/backend/api` | Cloudflare Worker API (Rust → WASM) |
| `packages/backend/health` | Health check Worker + dashboard |
| `packages/solana-programs/market` | Solana escrow program (deposit / lock / release / refund) |
| `packages/cli` | Native Rust CLI for market-program operations |

Cloudflare D1 is the off-chain database. Auth uses email/password and Google OpenID, with token signing handled in-Worker.

## See It In Action

| Surface | URL |
| --- | --- |
| Landing | <https://groupshop.org> |
| Health dashboard | <https://health.groupshop.org> |
| API | <https://api.groupshop.org> |

The Phantom deposit flow on a product page exercises the full stack end-to-end: signed wallet-ownership message → backend co-signed deposit payload → on-chain escrow.

## Local Development

Prerequisites: Rust toolchain, [`task`](https://taskfile.dev/), Node.js + npm, [`trunk`](https://trunkrs.dev/), `worker-build`, `wasm-bindgen-cli`, Cloudflare `wrangler`, Solana CLI tools (`solana`, `solana-keygen`, `solana-test-validator`, `cargo-build-sbf`), and `spl-token`.

```bash
npm install
cargo install worker-build wasm-bindgen-cli
cp .env.example .env  # then fill in the values you need
task dev
```

`task dev` brings up the full local stack: Solana validator, program build watcher, on-chain bootstrap (program deploy + USDC mint), backend API, landing, admin, and health services. Open the landing app at <http://127.0.0.1:9082>.

For the Phantom deposit flow on local: add a custom Phantom RPC endpoint at `http://127.0.0.1:9086` and switch to it. Set `SOLANA_WALLET_DEV` in `.env` to your Phantom address — `task dev` will airdrop local SOL and test USDC on startup.

To fund or top up a wallet manually:

```bash
task solana-programs:fund-local-wallet -- "$SOLANA_WALLET_DEV"
```

## Verification

```bash
task lint                          # fmt + WASM checks + Solana program build
task solana-tests:integration-test # validator + deploy + deposit/lock/release/refund flow
```

## Devnet Funding

Switch Phantom to **Devnet** (Settings → Developer Settings → Change Network), then:

- **Devnet SOL** (transaction fees): <https://faucet.solana.com/>
- **Devnet USDC** (escrow deposits): <https://faucet.circle.com/> — Circle devnet USDC mint: `4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU`

## Deployment

Devnet Solana setup, then full deploy:

```bash
task solana-programs:create-devnet-deploy-keypair  # back this up — it's the upgrade authority
task solana-programs:devnet-deploy-info            # confirm deploy_pubkey is funded
task deploy-all
```

`task deploy-all` runs:

- `task solana-programs:deploy-devnet`
- `task db:migrations-apply-prod`
- `task api:put-prod-secrets`
- `task api:deploy`
- `task landing:deploy`
- `task admin:deploy`
- `task health:deploy`
- `task health-dashboard:deploy`

The deploy keypair is only the program upgrade authority. The backend runtime authority that co-signs escrow instructions is a separate keypair, uploaded as a Worker secret. To rotate it manually:

```bash
wrangler secret put SOLANA_AUTHORITY_KEYPAIR_JSON -c cloudflare/api/wrangler.jsonc --env prod
```

Never commit a Solana keypair JSON file or paste it into `.env`.

## Product Catalog Import

The catalog is sourced from a Qogita CSV export. Imports are additive (safe to re-run). Full refresh:

```bash
task deploy-all                       # ensure prod has the latest API
task supplier:wipe-catalog-prod       # clear products, brands, categories
task supplier:qogita-import-prod      # import (~15–20 min end-to-end)
```

Filters applied before upload: minimum MOQ of 5, max 10 products per category, products with placeholder/missing images excluded. To download a fresh CSV first:

```bash
task supplier:qogita-download
```

## Useful Commands

```bash
task dev
task lint
task solana-programs:bootstrap-local
task solana-programs:deploy-devnet
task solana-programs:id-market-local
task solana-programs:id-market-devnet
task solana-tests:integration-test
task cli:market -- --help
```

## Vetting

See [COPILOT Vetting](./docs/COPILOT-VETTING.md) for the Colosseum Copilot vetting results.
