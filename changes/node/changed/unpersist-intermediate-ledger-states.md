#ledger #node
# Unpersist intermediate ledger states during block construction

`apply_transaction` and `apply_system_transaction` now unpersist their predecessor state after persisting the new state, so within-block intermediate per-tx states drop to refcount 0 and become GC-eligible instead of piling up as GC roots forever. `post_block_update` double-persists its output (and `alloc_with_initial_state` likewise double-persists genesis), so post-block tips stay at refcount 1 across the next block's first apply — preserving history for RPC queries. Net effect: only post-block tips remain rooted; intermediate states no longer accumulate.

Issue: https://github.com/midnightntwrk/midnight-node/issues/1442
