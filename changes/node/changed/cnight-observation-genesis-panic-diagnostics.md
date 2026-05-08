#audit #hardening
# Improve cnight-observation genesis panic diagnostics

The genesis-build path for the cnight-observation pallet now reports the
chain-spec field name, the supplied byte length, and the maximum permitted
length when a value exceeds its bounded-vector cap, replacing four short
"expected" panic strings with actionable diagnostics for operators reading
a startup-failure log. The field paths match the camelCase JSON keys the
operator edits in chain-spec files (e.g.
`cNightObservation.config.addresses.<field>`), and the cap is read from the
destination `BoundedVec` type via `bound()` so the diagnostic and the
storage type cannot drift apart.

Future hardening: a follow-up will move chain-spec parsing out of genesis
build by typing `CNightAddresses` fields with `BoundedVec` directly. That
refactor changes the chain-spec JSON encoding and is therefore separate
from this diagnostic-only change.

PR: https://github.com/midnightntwrk/midnight-node/pull/1466
Issue: https://github.com/shieldedtech/shielded-security-engineering/issues/365
JIRA: https://shielded.atlassian.net/browse/PM-19896
