#runtime
# Wire WeightInfo with benchmarked weights for nine unmetered pallets

Replace placeholder `WeightInfo = ()` with proper benchmarked weights for five
pallets in the midnight-node runtime: `pallet_federated_authority`,
`pallet_federated_authority_observation`, `pallet_system_parameters`,
`pallet_session_validator_management`, and `pallet_timestamp`. Four additional
pallets (`pallet_grandpa`, `pallet_mmr`, `pallet_beefy_mmr`, `pallet_beefy`)
are documented as intentionally unweighted because their upstream weights
modules are not publicly exported or they have no `runtime-benchmarks` feature.
Re-benchmarked four weight files with STEPS=100, REPEAT=2 against
midnight-node-runtime.

Fixes: https://github.com/midnightntwrk/midnight-security/issues/113
PR: https://github.com/midnightntwrk/midnight-node/pull/1338
