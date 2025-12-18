#runtime
# Add Fee to TxAppliedDetails

Currently, the indexer is making an extra RPC request for every transaction to get the transaction fees. We can provide this data in the event, to reduce necessary RPC calls.

Ticket: https://shielded.atlassian.net/browse/PM-20972
PR: https://github.com/midnightntwrk/midnight-node/pull/390
