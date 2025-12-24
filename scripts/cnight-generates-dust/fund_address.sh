#!/usr/bin/env bash

# This script sends ADA from the faucet address to a specified Cardano address.
# Usage: ./fund_address.sh <recipient_address> <amount_lovelace>

set -e

export CARDANO_NODE_SOCKET_PATH=node.socket
export CARDANO_NODE_NETWORK_ID=42

# Paths
FAUCET_ADDR_FILE="local-environment/src/networks/local-env/configurations/cardano/keys/funded_address.addr"
FAUCET_SKEY_FILE="local-environment/src/networks/local-env/configurations/cardano/keys/funded_address.skey"
CNIGHT_POLICY_FILE="scripts/cnight-generates-dust/cnight_policy.plutus"
CNIGHT_POLICY_HASH_FILE="scripts/cnight-generates-dust/cnight_policy.hash"

# Read faucet address
FAUCET_ADDR=$(< "$FAUCET_ADDR_FILE")

# Recipient address and amount
RECIPIENT_ADDR="$1"
AMOUNT="$2"

if [[ -z "$RECIPIENT_ADDR" || -z "$AMOUNT" ]]; then
  echo "Usage: $0 <recipient_address> <amount_lovelace>"
  exit 1
fi


# Query UTXOs for faucet address in JSON format
cardano-cli query utxo \
  --address "$FAUCET_ADDR" \
  --out-file utxos.json


# Pick a UTXO with at least 10 ADA (10_000_000 lovelace) for tx-in
TXIN_UTXO=$(jq -r 'to_entries[] | select(.value.value.lovelace >= 10000000) | .key' utxos.json | head -n 1)
if [[ -z "$TXIN_UTXO" ]]; then
  echo "No UTXO with at least 10 ADA (10000000 lovelace) found for tx-in."
  exit 1
fi

# Pick a different UTXO with at least 5 ADA (5_000_000 lovelace) for collateral
COLLATERAL_UTXO=$(jq -r --arg txin "$TXIN_UTXO" 'to_entries[] | select(.key != $txin and .value.value.lovelace >= 5000000) | .key' utxos.json | head -n 1)
if [[ -z "$COLLATERAL_UTXO" ]]; then
  echo "No UTXO with at least 5 ADA (5000000 lovelace) found for collateral."
  exit 1
fi

# Build transaction
cardano-cli conway transaction build \
  --tx-in "$TXIN_UTXO" \
  --tx-in-collateral "$COLLATERAL_UTXO" \
  --tx-out "$RECIPIENT_ADDR+1500000 lovelace + $AMOUNT $(< $CNIGHT_POLICY_HASH_FILE)" \
  --mint="$AMOUNT $(< $CNIGHT_POLICY_HASH_FILE)" \
  --mint-script-file $CNIGHT_POLICY_FILE \
  --mint-redeemer-value "{}" \
  --change-address "$FAUCET_ADDR" \
  --out-file fund-address.tx

# Sign transaction
cardano-cli conway transaction sign \
  --tx-file fund-address.tx \
  --signing-key-file "$FAUCET_SKEY_FILE" \
  --out-file fund-address.signed.tx

# Submit transaction
cardano-cli conway transaction submit \
  --tx-file fund-address.signed.tx \

# Print transaction ID
cardano-cli conway transaction txid --tx-file fund-address.signed.tx
