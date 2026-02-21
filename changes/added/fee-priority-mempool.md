#node #client
# Prioritise transactions in mempool by dust fee

Uses the transaction's gas cost (expected dust fee) as the ValidTransaction priority, so Substrate's pool naturally orders and evicts by fee. Higher-fee transactions are now preferred during block building and pool eviction under load.

PR: https://github.com/midnightntwrk/midnight-node/pull/741
Ticket: https://shielded.atlassian.net/browse/PM-21985
