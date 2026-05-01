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

/// Buyer-initiated refund. Same shape as deposit: backend pre-attaches
/// the authority signature and ships full transaction bytes; we
/// `Transaction.from()` to preserve the message and let Phantom sign
/// the buyer slot.
export async function phantomSignAndSendEscrowRefund(payloadJson) {
  const payload = JSON.parse(payloadJson);
  const provider = getProvider();
  const connectionKey = await provider.connect();
  const buyer = new PublicKey(connectionKey.publicKey.toString());
  const connection = new Connection(payload.rpc_url, "confirmed");

  if (buyer.toBase58() !== payload.wallet_address) {
    throw new Error("Connected Phantom wallet does not match the wallet that made the deposit");
  }

  const transaction = Transaction.from(base64ToBytes(payload.transaction_base64));
  const signedTransaction = await provider.signTransaction(transaction);
  const signature = await connection.sendRawTransaction(signedTransaction.serialize(), {
    preflightCommitment: "confirmed",
  });
  await connection.confirmTransaction(signature, "confirmed");
  return signature;
}

export async function phantomSignAndSendEscrowDeposit(payloadJson) {
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
  // populates the cached internal `_message`, so when Phantom serializes the
  // transaction during signing it reuses the exact bytes the authority signed
  // — no re-compilation, no signature mismatch.
  const transaction = Transaction.from(base64ToBytes(payload.transaction_base64));

  const signedTransaction = await provider.signTransaction(transaction);

  const signature = await connection.sendRawTransaction(signedTransaction.serialize(), {
    preflightCommitment: "confirmed",
  });
  await connection.confirmTransaction(signature, "confirmed");
  return signature;
}
