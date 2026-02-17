#performance #sync
# Adaptive sync verifier to accelerate block sync

Replace the partner-chains `AuraVerifier` import queue with an `AdaptiveVerifier` that skips the expensive Postgres-backed `check_inherents` for blocks far from the chain tip (>1000 slots behind). Full inherent verification resumes automatically when near the tip. Additional optimizations: parallelized inherent data queries via `futures::join!`, increased DB pool sizes (5 to 20), and batched `flush_storage()` every 1000 blocks.

## What `check_inherents` validates (the expensive part we skip)

The `check_inherents` runtime API creates inherent data from our **local Postgres/db-sync** and compares it against the inherent extrinsics already in the block. It is a "second opinion" check -- asking "does my local view of Cardano agree with what the block author claimed?" Specifically it validates 6 inherent data providers:

| Inherent | What it checks | Failure mode |
|---|---|---|
| **McHash** (mainchain reference) | The Cardano block hash referenced by the sidechain block matches what our db-sync says is the stable block for that slot | `McStateReferenceInvalid`, `McStateReferenceRegressed` |
| **Ariadne** (authority selection) | The validator committee data matches our local computation from Cardano SPO registrations | `InvalidValidatorsHashMismatch` (fatal) |
| **CNight observation** (tDUST tokens) | Token UTXOs and next Cardano position match our db-sync data | `InherentError::Other` (data mismatch) |
| **Federated authority** (council/TC) | Council and technical committee members match our observation of Cardano | `CouncilMembersMismatch`, `TechnicalCommitteeMembersMismatch` |
| **Token bridge** | Bridge transfer data matches our db-sync | `IncorrectInherent` (fatal) |
| **Timestamp** | Block timestamp within acceptable range | Standard substrate check |

Every one of these queries Postgres. That is the ~50-70ms per block cost.

## What still runs even when we skip `check_inherents`

**`execute_block` always runs during import.** The Substrate client calls `Core::execute_block()` for every block it imports (the verifier runs *before* import; it does not control whether the runtime executes the block). This means:

1. **All inherent extrinsics are fully executed** -- The runtime applies every extrinsic in the block, including inherents. Each pallet's dispatch logic runs. If an inherent extrinsic is malformed or internally inconsistent, execution fails and the block is rejected.

2. **State root verification** -- After executing all extrinsics, the runtime computes the resulting state root and compares it to the block header's `state_root`. Any discrepancy (from tampered extrinsics, missing inherents, wrong data) causes the block to be rejected.

3. **Mandatory inherent presence** -- Inherent extrinsics have `DispatchClass::Mandatory`. The runtime rejects blocks missing required inherents.

4. **Aura seal verification** -- The runtime's `execute_block` strips the seal and verifies the pre-digest matches an authorized author for that slot.

## Why it is safe for blocks far from the tip

The critical distinction:

- **`execute_block`** asks: *"Is this block internally valid? Do the extrinsics produce the claimed state?"*
- **`check_inherents`** asks: *"Does my local Postgres agree with what the block author put in these inherents?"*

For finalized blocks (blocks far from the tip), `check_inherents` is redundant because:

1. **GRANDPA finality** -- These blocks have been finalized by a 2/3+ supermajority of validators. Each of those validators ran both `check_inherents` AND `execute_block` against their own Postgres instances before voting to finalize. The network has already settled the question of whether this data is correct.

2. **`execute_block` catches structural invalidity** -- A block with missing inherents, wrong extrinsic format, or invalid state transitions will fail execution regardless. What we cannot catch without `check_inherents` is a block where the *data values* (e.g., which Cardano block hash is referenced) differ from our local db-sync -- but finality already guarantees those values are what 2/3+ of the network agreed on.

3. **Our local Postgres could be the one that is wrong** -- During initial sync, our db-sync might be slightly behind Cardano, have stale cache entries, or have different timing than what validators saw when they originally produced these blocks. Running `check_inherents` against lagging local data could cause *spurious* rejections of perfectly valid finalized blocks.

## What could theoretically go wrong

- **Long-range fork attack**: A malicious peer sends us blocks on a fake fork that was never finalized. We would import them without inherent checks. However:
  - GRANDPA will not finalize them (no justifications from 2/3+ validators)
  - `execute_block` still validates internal consistency
  - `ForkChoiceStrategy::LongestChain` means we will switch to the real chain when we see it

- **Corrupted db-sync**: If our Postgres has wrong data, we will only discover it when we get close to the tip and `check_inherents` starts running. This is acceptable since the blocks we synced are already finalized by the network.

## Protection summary

| Protection | Always runs | Skipped during fast sync |
|---|---|---|
| Aura seal extraction | Yes | -- |
| `execute_block` (full runtime execution) | Yes | -- |
| State root verification | Yes | -- |
| Mandatory inherent presence | Yes | -- |
| GRANDPA finalization | Yes | -- |
| `check_inherents` (Postgres comparison) | -- | Yes (resumes within 1000 slots of tip) |

## All optimizations

| Optimization | File | Description |
|---|---|---|
| **AdaptiveVerifier** | `node/src/service.rs` | Skips `check_inherents` during sync, full verification near tip |
| **Parallelized inherent data queries** | `node/src/inherent_data.rs` | 4-5 independent Postgres queries in `ProposalCIDP` and `VerifierCIDP` run concurrently via `futures::join!` instead of sequentially |
| **Increased Postgres pool sizes** | `node/src/main_chain_follower.rs` | All connection pools increased from 5 to 20 to support parallel queries without contention |
| **Batched ledger flush** | `pallets/midnight/src/lib.rs` | `flush_storage()` runs every 1000 blocks instead of every block, reducing ParityDB I/O during sync |
