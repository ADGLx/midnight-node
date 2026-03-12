#node
# Include `chain` field in `SubstrateCfg::argv()`

`SubstrateCfg::argv()` builds the argument vector used to parse node CLI arguments. It
included `args` and `append_args` but silently omitted the `chain` field. This caused
`chain` to be ignored when `argv()` was used as the sole source for `RunCmd` (as happened
in 0.22.0-rc.8), leading nodes to start with an empty chain ID, fall back to building a
chain spec from scratch, and panic when trying to JSON-parse a binary genesis block file.

The fix adds `--chain <value>` to the front of the argv vector when `chain` is set, so
the field is honoured regardless of which code path consumes `argv()`.

PR: https://github.com/midnightntwrk/midnight-node/pull/TBD
