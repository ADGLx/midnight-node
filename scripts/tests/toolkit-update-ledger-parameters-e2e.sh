#!/usr/bin/env bash

# This file is part of midnight-node.
# Copyright (C) 2025 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
# Licensed under the Apache License, Version 2.0 (the "License");
# You may not use this file except in compliance with the License.
# You may obtain a copy of the License at
# http://www.apache.org/licenses/LICENSE-2.0
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

set -euxo pipefail

NODE_IMAGE="$1"
TOOLKIT_IMAGE="$2"
RNG_SEED="0000000000000000000000000000000000000000000000000000000000000037"

echo "🎯 Running Toolkit E2E test"
echo "🧱 NODE_IMAGE: $NODE_IMAGE"
echo "🧱 TOOLKIT_IMAGE: $TOOLKIT_IMAGE"

# Ensure Docker network exists
docker network create ledger-params-e2e-net || true

# Start node in background
echo "🚀 Starting node container..."
docker run -d --rm \
  --name midnight-node \
  --network ledger-params-e2e-net \
  -e CFG_PRESET=dev \
  -e SIDECHAIN_BLOCK_BENEFICIARY="04bcf7ad3be7a5c790460be82a713af570f22e0f801f6659ab8e84a52be6969e" \
  "$NODE_IMAGE"

cleanup() {
    echo "🛑 Killing node container..."
    docker container stop midnight-node || true
    docker network rm ledger-params-e2e-net || true
}
# --- Always-cleanup: runs on success, error, or interrupt ---
trap cleanup EXIT

echo "⏳ Waiting for node to boot... (allow at least 2 blocks to be produced)"
sleep 20

# Run toolkit commands
echo "📦 Running toolkit tests..."

echo "Get version for toolkit"
docker run --rm -e RUST_BACKTRACE=1 --network ledger-params-e2e-net "$TOOLKIT_IMAGE" version

current_parameters=$(
    docker run --rm -e RESTORE_OWNER="$(id -u):$(id -g)" -e RUST_BACKTRACE=1 --network ledger-params-e2e-net "$TOOLKIT_IMAGE" \
        show-ledger-parameters -r ws://midnight-node:9944 --serialize
)

docker run --rm -e RESTORE_OWNER="$(id -u):$(id -g)" -e RUST_BACKTRACE=1 --network ledger-params-e2e-net "$TOOLKIT_IMAGE" \
    update-ledger-parameters -r ws://midnight-node:9944 -t //Alice -t //Bob -c //Dave -c //Eve --c-to-m-bridge-min-amount 2000

new_parameters=$(
    docker run --rm -e RESTORE_OWNER="$(id -u):$(id -g)" -e RUST_BACKTRACE=1 --network ledger-params-e2e-net "$TOOLKIT_IMAGE" \
        show-ledger-parameters -r ws://midnight-node:9944 --serialize
)

if [ "$current_parameters" != "$new_parameters" ]; then
  echo "✅ Ledger parameters update successful"
else
  echo "❌ Ledger parameters update failed"
  exit 1
fi

echo "✅ Toolkit E2E"
