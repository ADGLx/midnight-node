#node
# Always register benchmark host functions in RuntimeExecutor

Unify the HostFunctions type alias to always include frame_benchmarking host functions,
regardless of the runtime-benchmarks Cargo feature. This prevents consensus divergence
between nodes compiled with different feature sets.

PR: https://github.com/midnightntwrk/midnight-node/pull/1070
Ticket: https://shielded.atlassian.net/browse/PM-19965
