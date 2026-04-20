#!/bin/bash

# Default base path from Docker ENV
DEFAULT_BASE_PATH="$BASE_PATH"

# Parse arguments to find --base-path
PARSED_BASE_PATH=""
prev_arg=""
for arg in "$@"; do
    if [[ "$arg" == --base-path=* ]]; then
        # Extract value after = sign
        PARSED_BASE_PATH="${arg#*=}"
    elif [[ "$prev_arg" == "--base-path" ]]; then
        # Handle --base-path <value> format
        PARSED_BASE_PATH="$arg"
    fi
    prev_arg="$arg"
done

# Use default if not specified
if [ -z "$PARSED_BASE_PATH" ]; then
    FINAL_BASE_PATH="$DEFAULT_BASE_PATH"
else
    FINAL_BASE_PATH="$PARSED_BASE_PATH"
fi

# Create directories and set permissions if they don't exist
if [ ! -d "$FINAL_BASE_PATH" ]; then
    mkdir -p "$FINAL_BASE_PATH"
fi

# Auto-pick the cNIGHT observation snapshot based on which *Cardano* network
# the Midnight CFG_PRESET observes. There are many Midnight presets (devnet,
# govnet, qanet, …) but only three Cardano networks ever (mainnet, preprod,
# preview), so the snapshot files live in /res/cnight-observations/<net>.bin
# and every preset maps onto one of those three. Operator can override with
# CNIGHT_OBSERVATION_FILE + CARDANO_NETWORK_MAGIC directly.
if [ -z "${CNIGHT_OBSERVATION_FILE+x}" ]; then
    case "$CFG_PRESET" in
        mainnet)
            CARDANO_NET=mainnet
            : "${CARDANO_NETWORK_MAGIC:=764824073}"
            ;;
        preprod|devnet|govnet|perfnet|qanet|ddosnet|guardnet|dev|node-dev-01)
            CARDANO_NET=preprod
            : "${CARDANO_NETWORK_MAGIC:=1}"
            ;;
        preview)
            CARDANO_NET=preview
            : "${CARDANO_NETWORK_MAGIC:=2}"
            ;;
        *)
            CARDANO_NET=
            ;;
    esac
    if [ -n "$CARDANO_NET" ]; then
        CANDIDATE=/res/cnight-observations/${CARDANO_NET}.bin
        if [ -f "$CANDIDATE" ]; then
            export CNIGHT_OBSERVATION_FILE="$CANDIDATE"
            export CARDANO_NETWORK_MAGIC
            echo "entrypoint: using cNIGHT snapshot $CNIGHT_OBSERVATION_FILE (preset=$CFG_PRESET, cardano=$CARDANO_NET, magic=$CARDANO_NETWORK_MAGIC)"
        fi
    fi
fi

# Now run as appuser
exec /midnight-node "$@"
