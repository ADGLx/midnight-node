# D Parameter API Endpoint

Added new Runtime API and RPC endpoint to expose D Parameter visibility for
external tools and integrators. The `get_d_parameter()` API returns the D
Parameter from on-chain governance when available, or `None` if sourced from
inherent data.

This prepares infrastructure for `pallet-system-parameters` integration.

Ticket: https://shielded.atlassian.net/browse/PM-20993
PR: https://github.com/midnightntwrk/midnight-node/pull/382

