#runtime
# Replace on_finalize panic with graceful error handling

Replace the `.expect()` on `LedgerApi::post_block_update` in
`pallet-midnight::on_finalize` with a `match` expression that logs the error at
`log::error!` level (including block number and error variant) and preserves the
previous state key when the post-block update fails. Block production continues
on the last known-good state instead of halting the network on transient or
recoverable ledger errors. Addresses security audit finding R-012.

PR: https://github.com/midnightntwrk/midnight-node/pull/1438
JIRA: https://shielded.atlassian.net/browse/PM-21802
