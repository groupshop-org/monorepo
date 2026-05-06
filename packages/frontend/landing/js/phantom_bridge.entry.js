import { Connection, PublicKey, Transaction } from "@solana/web3.js";

function getProvider() {
  const provider = window.phantom?.solana ?? window.solana;
  if (!provider?.isPhantom) {
    throw new Error("Phantom provider not found");
  }
  return provider;
}

function base64ToBytes(value) {
  const normalized = value.replace(/-/g, "+").replace(/_/g, "/");
  const padded = normalized + "=".repeat((4 - (normalized.length % 4 || 4)) % 4);
  const decoded = window.atob(padded);
  return Uint8Array.from(decoded, (char) => char.charCodeAt(0));
}

function bytesToBase64(bytes) {
  let raw = "";
  for (const byte of bytes) {
    raw += String.fromCharCode(byte);
  }
  return window.btoa(raw);
}

export function phantomIsAvailable() {
  return Boolean(window.phantom?.solana?.isPhantom || window.solana?.isPhantom);
}

export async function phantomConnect() {
  const provider = getProvider();
  const result = await provider.connect();
  return result.publicKey.toString();
}

export async function phantomSignMessage(message) {
  const provider = getProvider();
  await provider.connect();
  const encoded = new TextEncoder().encode(message);
  const result = await provider.signMessage(encoded, "utf8");
  const signature = result?.signature ?? result;
  return bytesToBase64(signature);
}

async function simulateBeforeWalletSignature(connection, transaction) {
  // Legacy `Transaction` uses the deprecated overload:
  // simulateTransaction(transaction, signers?, includeAccounts?). Passing the
  // VersionedTransaction config object throws "Invalid arguments" in web3.js.
  // With no signers, web3.js submits the simulation with sigVerify disabled.
  const result = await connection.simulateTransaction(transaction);
  if (result.value.err) {
    throw new Error(`Escrow transaction simulation failed: ${JSON.stringify(result.value.err)}`);
  }
}

/// Buyer-initiated refund. The backend ships transaction bytes with both
/// signature slots empty; Phantom signs the buyer slot first, then Rust sends
/// the partially signed bytes back to the backend for authority signing.
export async function phantomSignEscrowRefundTransaction(payloadJson) {
  const payload = JSON.parse(payloadJson);
  const provider = getProvider();
  const connectionKey = await provider.connect();
  const buyer = new PublicKey(connectionKey.publicKey.toString());
  const connection = new Connection(payload.rpc_url, "confirmed");

  if (buyer.toBase58() !== payload.wallet_address) {
    throw new Error("Connected Phantom wallet does not match the wallet that made the deposit");
  }

  const transaction = Transaction.from(base64ToBytes(payload.transaction_base64));
  await simulateBeforeWalletSignature(connection, transaction);
  const signedTransaction = await provider.signTransaction(transaction);
  return bytesToBase64(
    signedTransaction.serialize({
      requireAllSignatures: false,
      verifySignatures: false,
    }),
  );
}

export async function phantomSignEscrowDepositTransaction(payloadJson) {
  const payload = JSON.parse(payloadJson);
  const provider = getProvider();
  const connectionKey = await provider.connect();
  const buyer = new PublicKey(connectionKey.publicKey.toString());
  const connection = new Connection(payload.rpc_url, "confirmed");

  if (buyer.toBase58() !== payload.wallet_address) {
    throw new Error("Connected Phantom wallet does not match the approved wallet");
  }

  const buyerAta = new PublicKey(payload.buyer_associated_token_account);
  const buyerAtaInfo = await connection.getAccountInfo(buyerAta, "confirmed");
  if (!buyerAtaInfo) {
    throw new Error(
      payload.network === "local"
        ? `No local USDC token account was found for this wallet. Run: task solana-programs:fund-local-wallet -- ${payload.wallet_address}`
        : "No USDC token account was found for this wallet on the configured Solana network",
    );
  }

  const balance = await connection.getTokenAccountBalance(buyerAta, "confirmed");
  const available = BigInt(balance.value.amount);
  const required = BigInt(payload.total_amount_base_units);
  if (available < required) {
    throw new Error(
      payload.network === "local"
        ? `Insufficient local USDC. Wallet has ${balance.value.uiAmountString ?? balance.value.amount} but needs ${(Number(payload.total_amount_base_units) / 1_000_000).toFixed(6)}. Run: task solana-programs:fund-local-wallet -- ${payload.wallet_address}`
        : `Insufficient USDC balance for deposit. Wallet has ${balance.value.uiAmountString ?? balance.value.amount}`,
    );
  }

  // Deserialize the backend-built transaction directly. Transaction.from()
  // preserves the exact message bytes the backend approved, so Phantom signs
  // the buyer slot without recompiling or reordering accounts.
  const transaction = Transaction.from(base64ToBytes(payload.transaction_base64));

  await simulateBeforeWalletSignature(connection, transaction);
  const signedTransaction = await provider.signTransaction(transaction);
  return bytesToBase64(
    signedTransaction.serialize({
      requireAllSignatures: false,
      verifySignatures: false,
    }),
  );
}
