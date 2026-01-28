#earthly

# Add Earthly target for reserve contracts CLI

Adds an Earthly target to generate contract info JSON from the `midnight-reserve-contracts` CLI.
This provides script hashes and addresses for Aiken contracts needed in genesis generation.

Usage:
```bash
earthly --secret GH_TOKEN +reserve-contracts-info --NETWORK=preview
```

PR: https://github.com/midnightntwrk/midnight-node/pull/553
JIRA: https://shielded.atlassian.net/browse/PM-21429
