
set -euxo pipefail

compiled_contract="util/toolkit-js/dist_dapp/"
outdir="out_dapp"
compactc_bin="./compactc_v0.28.0-rc.1_x86_64-unknown-linux-musl/compactc"
toolkit_bin="./target/release/midnight-node-toolkit"
state_filename="contract_state.mn"
config_file="util/toolkit-js/dapp.config.ts"

call_intent_filename="call_intent.bin"
call_tx_filename="call_tx.mn"
call_private_state_filename="call_state.json"

initial_private_state_filename="initial_state.json"

deploy_intent_filename="deploy.bin"
deploy_tx_filename="deploy.mn"

mkdir -p $outdir

if [ ! -d $compiled_contract ]; then
    $compactc_bin midnight-wallet-dapp/src/contract/contracts/unshielded-demo.compact $compiled_contract
fi

coin_public=$(
    $toolkit_bin \
    show-address \
    --network undeployed \
    --seed 0000000000000000000000000000000000000000000000000000000000000001 \
    --coin-public
)

echo "Generate deploy intent"
"$toolkit_bin" \
    generate-intent deploy -c "$config_file" \
    --toolkit-js-path "$PWD/util/toolkit-js" \
    --coin-public "$coin_public" \
    --output-intent "$outdir/$deploy_intent_filename" \
    --output-private-state "$outdir/$initial_private_state_filename" \
    --output-zswap-state "$outdir/temp.json"

test -f "$outdir/$deploy_intent_filename"

echo "Generate deploy tx"
"$toolkit_bin" \
    send-intent \
    --intent-file "$outdir/$deploy_intent_filename" \
    --compiled-contract-dir $compiled_contract \
    --to-bytes --dest-file "$outdir/$deploy_tx_filename"

echo "Send deploy tx"
"$toolkit_bin" generate-txs --src-file $outdir/$deploy_tx_filename -r 1 send

contract_address=$(
"$toolkit_bin" \
    contract-address \
    --src-file $outdir/$deploy_tx_filename
)

echo "Get contract state"
"$toolkit_bin" \
    contract-state \
    --contract-address $contract_address \
    --dest-file $outdir/$state_filename

test -f "$outdir/$state_filename"

user_address=$(
    "$toolkit_bin" \
        show-address \
        --network undeployed \
        --seed 00..10 \
        --unshielded
)

echo "Generate circuit call intent"
"$toolkit_bin" \
    generate-intent circuit -c "$config_file" \
    --toolkit-js-path "$PWD/util/toolkit-js" \
    --coin-public "$coin_public" \
    --input-onchain-state "$outdir/$state_filename" \
    --input-private-state "$outdir/$initial_private_state_filename" \
    --contract-address $contract_address \
    --output-intent "$outdir/$call_intent_filename" \
    --output-private-state "$outdir/$call_private_state_filename" \
    --output-zswap-state "$outdir/temp.json" \
    sendToUser \
    333 \
    "{ bytes: '$user_address' }"

echo "Generate & send circuit call tx"
"$toolkit_bin" \
    send-intent \
    --intent-file "$outdir/$call_intent_filename" \
    --compiled-contract-dir "$compiled_contract"
