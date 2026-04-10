# Tooling

## Install

- Install JS dependencies with `npm install`
- Install `worker-build` with `cargo install worker-build`
- Install `wasm-bindgen-cli` with `cargo install wasm-bindgen-cli`

## Main Tasks

- `task lint` — formatting and compile checks
- `task everything-dev` — run API and landing locally
- `task api:dev` — run the backend worker locally
- `task landing:dev` — run the landing frontend locally
- `task everything-deploy` — deploy API and landing

## Database (D1)

Migration SQL files live in `cloudflare/api/db/migrations/`.

During development, schema changes should be made by editing the existing migration files in place rather than creating throwaway incremental migrations. After editing, run:

```sh
task db:recreate-dev
```

This drops and recreates the local D1 database from scratch using the updated migrations.

Only create a new migration file when the change must be applied to an already-deployed database without wiping data.

Keep seed data in migrations in sync with the corresponding Rust enums. If a SQL seed row uses numeric IDs, the Rust discriminants must match exactly.
