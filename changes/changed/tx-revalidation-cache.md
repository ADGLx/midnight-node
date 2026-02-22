#node #performance
# Replace soft tx cache with revalidation-based validation cache

Removed the soft transaction cache and introduced revalidation-based caching:
cached `VerifiedTransaction` entries are reused when ledger state changes by
revalidating against a `RevalidationReference` instead of re-running full ZK
proof verification. Adds cache metrics (miss, strict hit, revalidation hit) and
tests covering the full validation lifecycle.

PR: https://github.com/midnightntwrk/midnight-node/pull/744
JIRA: 
    https://shielded.atlassian.net/browse/PM-21736
    https://shielded.atlassian.net/browse/PM-18691
