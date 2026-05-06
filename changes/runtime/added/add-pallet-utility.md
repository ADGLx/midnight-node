#runtime
# Add pallet-utility to runtime

Added `pallet-utility` from polkadot-sdk to the runtime at pallet index 20 to allow Governance to batch calls. Without this, two governance actions can happen at the same block but one can get voted in while the other does not, or one can work while the other fails, leaving the chain in a potentially broken state. This allows multiple extrinsics to happen atomically.

In addition, batching governance actions reduces burden on node operators. An example, we'd like to update both the transaction cost parameters and perform a runtime upgrade.

PR: https://github.com/midnightntwrk/midnight-node/pull/463
Issue: https://github.com/midnightntwrk/midnight-node/issues/1143
