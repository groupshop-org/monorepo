# Solana Programs

This document describes the on-chain programs that back Groupshop's group-buying escrow flow. It is the design reference — update it when any decision below changes.

For the local validator / CLI / build workflow, see [`docs/ai/solana-dev.md`](ai/solana-dev.md).

## Programs

| Program | Path | Role |
|---|---|---|
| `groupshop-contract-market` | `packages/solana-programs/market` | Per-product escrow: pools buyer USDC, binds each deposit to a backend `UserId`, releases to a backend-controlled wallet, supports refunds |

Currently one program. Additional programs (e.g. loyalty, reputation) will get their own crates and their own section here.

## Core Concepts

### Group-buying escrow

A **product pool** collects USDC deposits from many buyers. Each buyer pays two conceptual amounts at deposit time:

1. **Product share** — their contribution toward the group purchase
2. **Shipping share** — their individual shipping cost

Both amounts are held in a single USDC vault owned by a program-derived account (PDA), but tracked separately on the buyer's per-participant record.

When the backend is ready to place the real-world wholesale order (e.g. via Qogita), it calls **Release**, which transfers the entire vault balance to a backend-controlled destination wallet. From that point funds are off-chain and subject to normal banking rails.

If the pool fails before release, the backend flips the pool into **Refunding** and returns each participant their exact recorded amount from the vault.

If the order fails _after_ release, the backend must first re-seed the vault with USDC (a plain SPL transfer — no program instruction needed) and then enter Refunding, at which point participants claim normally.

### Identity: `UserId` bound to wallet

The backend's `UserId` is a 32-byte hash (see [`backend-shared/src/id.rs`](../packages/backend/backend-shared/src/id.rs)). Buyers pay from a Solana wallet that is not intrinsically linked to any `UserId`. The link is established at deposit time by requiring the **backend authority keypair to co-sign the deposit transaction**.

The backend will only co-sign when it has verified, off-chain, that:

- the wallet actually belongs to the `UserId` (normal session auth), and
- the `UserId` is allowed to join this pool at the stated price.

Because the backend is a required signer on every `Deposit`, any `(user_id, wallet, amounts)` tuple that lands on-chain carries the backend's attestation. The program itself does no ed25519 verification — it just checks `authority.is_signer()`.

For wallet UX, buyer-facing transactions are signed in Phantom's recommended order: the browser asks Phantom to sign the buyer slot first, then sends the partially signed transaction back to the authenticated backend. The backend verifies the exact message bytes against the approved deposit or refund recipe, adds the authority signature, and submits the fully signed transaction. This keeps the two-signer security model while avoiding the Phantom warning pattern caused by shipping a transaction to Phantom with another signature already attached.

A wallet ownership proof (e.g. a standalone signed message) is a **separate** artifact captured by the backend off-chain; it is not required by the program.

### Product identity: SHA-256 of the slug

`ProductId` is a variable-length slug, but PDA seeds are capped at 32 bytes. On-chain we use `product_hash = SHA-256(product_id_slug_bytes)`. The hash is:

- used as a PDA seed for the pool, vault, and participation accounts
- stored in the D1 `product_catalog` row next to the slug, so the backend can correlate on-chain accounts back to products with a single indexed lookup

### Backend authority

All privileged actions (initialize, lock, release, enter-refund, claim-refund) require the backend authority keypair to sign. The authority address is stored on the `Pool` account at initialization and cannot be rotated by the program (rotation, if ever needed, is a follow-up instruction).

## Accounts

### `Pool` PDA

Seeds: `[b"pool", product_hash]`

| Field | Type | Purpose |
|---|---|---|
| `version` | `u8` | Schema version for future migrations |
| `bump` | `u8` | PDA bump for self |
| `vault_bump` | `u8` | PDA bump for the vault authority |
| `status` | `u8` | `Open = 0`, `Locked = 1`, `Released = 2`, `Refunding = 3` |
| `authority` | `[u8; 32]` | Backend authority pubkey |
| `product_hash` | `[u8; 32]` | SHA-256 of product slug |
| `usdc_mint` | `[u8; 32]` | USDC mint for this pool (fixed at init) |
| `vault` | `[u8; 32]` | SPL token account address (convenience; derivable) |
| `product_total` | `u64` | Cumulative product share deposited (informational) |
| `shipping_total` | `u64` | Cumulative shipping share deposited (informational) |
| `refundable_outstanding` | `u64` | Sum of `(product + shipping)` across participations that have not yet been refunded. Incremented by `Deposit`, decremented by `ClaimRefund`. Used by `EnterRefundMode` to assert the vault is adequately funded. |
| `participant_count` | `u32` | Number of `Participation` records (informational) |
| `created_at` | `i64` | Unix seconds |
| `updated_at` | `i64` | Unix seconds |

### `Vault` (SPL token account)

Seeds for the **authority** PDA: `[b"vault-auth", product_hash]`.
The vault itself is an SPL token account whose owner is the vault-authority PDA. It holds USDC only — never any other mint.

Using a separate PDA as the token account owner (rather than the `Pool` PDA) keeps CPI signer-seed logic simple and isolates token-level permissions from pool state.

### `Participation` PDA

Seeds: `[b"part", product_hash, user_id]`

| Field | Type | Purpose |
|---|---|---|
| `version` | `u8` | Schema version |
| `bump` | `u8` | PDA bump |
| `refunded` | `u8` | `0` or `1` |
| `_pad` | `u8` | Alignment |
| `pool` | `[u8; 32]` | Owning pool PDA |
| `user_id` | `[u8; 32]` | Backend-issued `UserId` |
| `wallet` | `[u8; 32]` | Wallet that paid |
| `product_amount` | `u64` | Product share in USDC base units (6 decimals) |
| `shipping_amount` | `u64` | Shipping share in USDC base units |
| `deposited_at` | `i64` | Unix seconds |

Keying the PDA by `user_id` (not wallet) guarantees a `UserId` cannot double-deposit from two wallets. A single wallet may still pay for multiple `UserId`s across separate deposits (e.g. a parent paying for kids).

## Instructions

All monetary amounts are USDC base units (6 decimals). Everywhere below, "USDC" means the backend-configured mint stored on `Pool.usdc_mint`.

### 1. `InitializePool`

Creates `Pool` and the vault token account for a new product.

**Signers:** backend authority (also pays rent)
**Inputs:** `product_hash: [u8; 32]`, `usdc_mint: [u8; 32]`
**Accounts:** authority, pool PDA, vault-authority PDA, vault token account, usdc mint, system program, token program, rent sysvar

### 2. `Deposit`

A buyer contributes their product + shipping shares to the pool. Creates the `Participation` PDA on first deposit; on subsequent deposits from the same `UserId`, adds to the existing `product_amount` / `shipping_amount` (a "top-up"). Only allowed while `pool.status == Open`.

Top-ups must come from the same wallet originally recorded on the `Participation`. If a buyer switches wallets, the backend refuses to co-sign.

**Signers:** buyer wallet (the payer), backend authority
**Inputs:** `user_id: [u8; 32]`, `product_amount: u64`, `shipping_amount: u64`
**Accounts:** buyer wallet, buyer USDC token account, vault token account, pool PDA, participation PDA, authority, system program, token program

**Checks:**

- `pool.status == Open`
- `authority` is signer and matches `pool.authority`
- if `participation` already exists: `participation.wallet == buyer wallet` and `participation.refunded == 0`
- SPL transfer of `product_amount + shipping_amount` USDC from buyer ATA to vault succeeds

**Effects:**

- `participation.product_amount += product_amount`
- `participation.shipping_amount += shipping_amount`
- `pool.product_total += product_amount`
- `pool.shipping_total += shipping_amount`
- `pool.refundable_outstanding += product_amount + shipping_amount`
- `pool.participant_count += 1` (first deposit only)

**Why the backend is a required co-signer.** The `(user_id, product_amount, shipping_amount)` triple is not verified by the program — the program trusts whatever the backend signs for. The backend computes the correct amounts off-chain (based on the product's current group price, shipping zone for the user, etc.) and includes them in the instruction data. The buyer signs to authorize the token transfer from their wallet; the backend signs to attest "yes, this `UserId` is this wallet, and this is what they owe right now."

**Worked example.** A buyer wants to join the pool for product `sku-abc` (hash `h`). The backend knows:

- current group price per unit = 25 USDC, buyer is taking 1 unit → `product_amount = 25_000_000`
- buyer's shipping zone cost = 4.50 USDC → `shipping_amount = 4_500_000`
- buyer is logged in as `UserId = U`, connected wallet = `W`

The backend constructs a transaction containing one `Deposit` instruction:

```
data: { user_id: U, product_amount: 25_000_000, shipping_amount: 4_500_000 }
signers: [W (buyer), authority (backend)]
```

The buyer's client signs it first in Phantom (authorizing their wallet). The backend then verifies the signed message, adds the authority signature (authorizing the identity binding and the amounts), and submits it. The program transfers `29_500_000` USDC from the buyer's ATA to the vault and writes a `Participation` PDA `[b"part", h, U]` recording `(pool=h, user_id=U, wallet=W, product_amount=25_000_000, shipping_amount=4_500_000)`. The buyer cannot lie about their `UserId` (backend wouldn't co-sign), and the backend cannot move the buyer's money without the buyer co-signing.

### 3. `LockPool`

Freezes new deposits ahead of release. `Open → Locked`.

**Signers:** authority
**Inputs:** none
**Accounts:** authority, pool PDA

### 4. `Release`

Transfers the entire vault balance to a destination USDC token account. `Locked → Released`.

**Signers:** authority
**Inputs:** none (destination is an account, not data)
**Accounts:** authority, pool PDA, vault-authority PDA, vault token account, **destination USDC token account**, token program

The destination is passed as an account at release time (not stored at init), giving the backend flexibility to choose which wallet pulls the funds when the wholesale order is actually placed.

### 5. `EnterRefundMode`

Marks the pool as refundable. `Open | Locked | Released → Refunding`.

**Signers:** authority
**Inputs:** none
**Accounts:** authority, pool PDA

If the previous state was `Released`, the backend is responsible for re-funding the vault with USDC via a plain SPL transfer _before_ calling `EnterRefundMode`. The instruction asserts `vault.balance >= pool.refundable_outstanding` at transition — so the backend cannot enter refund mode until the vault is adequately seeded.

### 6. `ClaimRefund`

Transfers one participant's full recorded amount (`product_amount + shipping_amount`) from the vault to their wallet's USDC ATA. Marks `refunded = 1`.

**Signers:** authority
**Inputs:** `user_id: [u8; 32]`
**Accounts:** authority, pool PDA, participation PDA, vault-authority PDA, vault token account, **destination USDC token account (must match `participation.wallet`'s ATA)**, token program

Backend-initiated only. Each participation can be refunded at most once.

**Effects:**

- transfers `participation.product_amount + participation.shipping_amount` USDC from vault to destination
- `participation.refunded = 1`
- `pool.refundable_outstanding -= participation.product_amount + participation.shipping_amount`

## State Machine

```
               Deposit(s)
               ───────────►
                              LockPool              Release
  [Open] ─────────────────► [Locked] ─────────────► [Released]
     │                         │                        │
     │ EnterRefundMode         │ EnterRefundMode        │ EnterRefundMode
     │                         │                        │ (backend re-seeds vault first)
     ▼                         ▼                        ▼
                         [Refunding]
                              │
                              │ ClaimRefund (per participant, once)
                              ▼
                         (terminal when all refunded)
```

`Refunding` is terminal from the program's perspective — there is no path back to `Open`.

## Token Program

USDC lives on the standard SPL Token program (not Token-2022). CPI calls use a lightweight pinocchio-compatible token helper; we do **not** pull `solana-program` or `spl-token` into the on-chain crate (they're heavy). If no suitable helper exists we'll hand-roll the CPI serialization inline — it's ~30 bytes.

## Backend Integration

- D1 `product_catalog` gains a `product_hash BLOB NOT NULL UNIQUE` column storing the 32-byte SHA-256 of the slug. Populated at product creation; never rewritten.
- D1 gains a table (name TBD) that mirrors on-chain participations for fast queries: `(product_hash, user_id, wallet, product_amount, shipping_amount, status, tx_sig)`. Source of truth is on-chain; D1 is a read-through cache kept fresh by backend workers.
- The backend authority keypair lives in Cloudflare secrets; never in source.

## Deferred

Intentionally out of scope for the first implementation. Listed here so we don't forget and so the trigger for revisiting is explicit.

- **Authority rotation.** No `SetAuthority` instruction. Revisit when there's a real operational need (e.g. key rotation policy, compromised key recovery).
- **Rent reclaim / account closure.** `Pool` and `Participation` PDAs stay resident forever in the first pass. Add a terminal `Closed` status plus `ClosePool` / `CloseParticipation` instructions in a later pass once the "all refunded / fully released" bookkeeping is proven in practice.
- **Multiple stablecoins.** `Pool.usdc_mint` is fixed at init, which handles devnet-vs-mainnet USDC naturally. Supporting other stablecoins (USDT, PYUSD) would mean separate pools per mint — no program changes needed, just backend product-routing.
