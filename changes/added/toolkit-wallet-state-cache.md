#toolkit
# Add file-based wallet and ledger state caching to toolkit

Introduces a two-tier file cache that persists ledger snapshots and per-wallet state across toolkit runs, eliminating the need to replay the full chain on every invocation. Ledger snapshots (~49MB, zstd-compressed) are stored once per block height and shared across wallets; per-wallet state (~5-15KB BSON) is keyed by seed hash. Atomic writes (write to `.tmp`, then rename) ensure crash safety.

Includes a trusted deserialization path that computes hashes in a single bottom-up pass for self-generated cache data, bypassing the two-pass security verification and cutting deserialization time by ~20s. Similarly, fast serialization calls `serialize_to_node_list()` once instead of twice, saving another ~20s.

Stale snapshot garbage collection reads only the first 26 bytes of BSON headers to extract block height without full deserialization.

New CLI flags: `--ledger-state-db <path>` to set the cache directory (default: `ledger_state_db`), and `--fetch-only-cached` for offline operation from a pre-populated cache.

PR:
- https://github.com/midnightntwrk/midnight-node/pull/820
- https://github.com/midnightntwrk/midnight-node/pull/939
JIRA: https://shielded.atlassian.net/browse/PM-22103
