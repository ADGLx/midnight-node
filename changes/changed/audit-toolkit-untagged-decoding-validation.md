#toolkit
# Add type-tag validation and trailing-bytes check to CLI parsers

CLI parsers for coin-public and contract-address now try tagged
deserialization first (validating the type tag) before falling back
to untagged decoding. All untagged decode paths now reject inputs
with unexpected trailing bytes. Existing untagged inputs continue
to work.

PR: https://github.com/midnightntwrk/midnight-node/pull/930
JIRA: PM-21967
