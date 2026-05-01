#!/usr/bin/env bash
set -euo pipefail

: "${SOLANA_WALLET:?SOLANA_WALLET is not set}"

until solana airdrop 2 "$SOLANA_WALLET" --url devnet; do
    echo "Airdrop failed, retrying in 5 seconds..."
    sleep 5
done

echo "Airdrop succeeded."
