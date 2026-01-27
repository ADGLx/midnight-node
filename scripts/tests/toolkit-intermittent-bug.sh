
set -euxo pipefail

RPC_URL="wss://rpc.qanet.dev.midnight.network"
TOOLKIT_BIN="./target/release/midnight-node-toolkit"

$TOOLKIT_BIN dust-balance \
    --seed 00..01 \
    -s "$RPC_URL"
