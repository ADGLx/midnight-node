#node #runtime
# Prevalidate post-block ledger updates during dispatch

Transaction and system transaction dispatch now verifies the resulting
post-block ledger update before persisting the applied ledger state. This lets
the runtime reject extrinsics whose block-finalization ledger update would fail,
instead of accepting a state transition that cannot be completed at the end of
the block.

PR: https://github.com/midnightntwrk/midnight-node/pull/1448
Issue: https://github.com/shieldedtech/shielded-security-engineering/issues/116
