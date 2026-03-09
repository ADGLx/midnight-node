# Toolkit Architecture

## Diagram 1: Component Overview

```mermaid
graph TB
    %% ─── CLI Layer ───
    subgraph CLI["CLI Layer"]
        direction LR
        cli_rs["cli.rs<br/>(Commands enum)"]
        commands["commands/"]
        fork_cmds["commands/fork/<br/>ledger_7.rs | ledger_8.rs<br/>common/ (dual-compiled)"]
        cli_rs --> commands
        commands --> fork_cmds
    end

    %% ─── TxGenerator ───
    subgraph TxGen["TxGenerator (tx_generator/mod.rs)"]
        direction LR
        source["Source (GetTxs)<br/>GetTxsFromFile<br/>GetTxsFromUrl"]
        builder["Builder (BuildTxs)<br/>to_versioned_builder()"]
        destination["Destination (SendTxs)<br/>SendTxsToFile<br/>SendTxsToUrl"]
        source -->|"SourceTransactions"| builder
        builder -->|"SerializedTxBatches"| destination
    end

    %% ─── Fetcher ───
    subgraph Fetcher["Fetcher (fetcher.rs, fetcher/)"]
        direction TB
        fetch_all["fetch_all()"]
        fetch_task["FetchTask<br/>(fetch_task.rs)"]
        compute_task["ComputeTask<br/>(compute_task.rs)"]
        runtimes["RuntimeVersion<br/>(runtimes.rs)<br/>V0_17_0..V0_22_0"]
        storage["FetchStorage<br/>(fetch_storage.rs)"]
        wallet_cache["WalletStateCaching<br/>(wallet_state_cache.rs)"]

        fetch_all --> fetch_task
        fetch_task --> compute_task
        compute_task --> runtimes
        compute_task --> storage
        fetch_all --> wallet_cache
    end

    subgraph StorageBackends["Storage Backends"]
        inmemory["InMemory"]
        redb["ReDB<br/>(redb_backend.rs)"]
        postgres["PostgreSQL<br/>(postgres_backend.rs)"]
    end

    %% ─── Builder Context ───
    subgraph BuilderCtx["Builder Context (tx_generator/builder/)"]
        direction TB
        fork_ctx["ForkAwareLedgerContext<br/>(enum: Ledger7 | Ledger8)"]
        dispatch["dispatch(f7, f8)"]
        v7_builders["ledger_7 builders<br/>(builders/ledger_7.rs)"]
        v8_builders["ledger_8 builders<br/>(builders/ledger_8.rs)"]
        common_builders["common/ builders<br/>(compiled twice via #path)"]
        prover_cfg["ProverConfig<br/>Local | Remote"]

        fork_ctx --> dispatch
        dispatch --> v7_builders
        dispatch --> v8_builders
        v7_builders -.-> common_builders
        v8_builders -.-> common_builders
        prover_cfg --> v7_builders
        prover_cfg --> v8_builders
    end

    %% ─── toolkit_js ───
    subgraph TJS["toolkit_js (toolkit_js/mod.rs)"]
        direction LR
        toolkit_js_struct["ToolkitJs"]
        deploy_cmd["Deploy"]
        circuit_cmd["Circuit"]
        maintain_cmd["Maintain"]
        toolkit_js_struct --> deploy_cmd
        toolkit_js_struct --> circuit_cmd
        toolkit_js_struct --> maintain_cmd
    end

    %% ─── Infrastructure ───
    subgraph Infra["Infrastructure"]
        client["MidnightNodeClient<br/>(client.rs)<br/>Subxt WebSocket"]
        sender["Sender<br/>(sender.rs)<br/>rate-limited submission"]
        serde_def["serde_def/<br/>SourceTransactions<br/>(.mn format)"]
        remote_prover_mod["RemoteProofServer<br/>(remote_prover.rs)"]
    end

    %% ─── Genesis ───
    genesis["genesis_generator.rs"]

    %% ─── External Systems ───
    subgraph External["External Systems"]
        node["Midnight Node<br/>(RPC :9944)"]
        proof_server["Proof Server<br/>(HTTP)"]
        toolkit_js_node["toolkit-js<br/>(Node.js process)"]
    end

    %% ─── Connections ───
    CLI --> TxGen
    CLI --> TJS
    CLI --> genesis

    source -->|"GetTxsFromUrl"| Fetcher
    storage --> StorageBackends
    Fetcher -->|"RPC"| client
    destination -->|"SendTxsToUrl"| sender

    builder --> BuilderCtx

    client -->|"WebSocket"| node
    sender -->|"WebSocket"| node
    remote_prover_mod -->|"HTTP"| proof_server
    TJS -->|"child process"| toolkit_js_node

    %% ─── Styles ───
    style Fetcher fill:#e3f2fd,stroke:#1565c0,color:#000
    style BuilderCtx fill:#efebe9,stroke:#795548,color:#000
    style TJS fill:#f3e5f5,stroke:#7b1fa2,color:#000
    style External fill:#fff,stroke:#999,stroke-dasharray: 5 5,color:#000
    style CLI fill:#e8f5e9,stroke:#2e7d32,color:#000
    style TxGen fill:#fff3e0,stroke:#e65100,color:#000
    style Infra fill:#fafafa,stroke:#616161,color:#000
    style StorageBackends fill:#e3f2fd,stroke:#1565c0,color:#000
```

### Legend

| Color | Component | Re-implements |
|-------|-----------|---------------|
| Blue | Fetcher, Storage Backends | Indexer -- parallel block fetching, extraction, and caching |
| Brown | Builder Context | Wallet -- ledger context management, wallet state, proof generation |
| Purple | toolkit_js | midnight-js -- custom contract Deploy/Circuit/Maintain via Node.js bridge |
| Green | CLI Layer | -- |
| Orange | TxGenerator | -- |
| Grey | Infrastructure, External | -- |

---

## Diagram 2: Fetcher Pipeline Detail

```mermaid
graph LR
    subgraph JobPusher["Job Pusher"]
        pusher["for min in (0..max)<br/>.step_by(BLOCKS_PER_JOB)"]
    end

    fetch_job_ch[/"fetch_job channel<br/>(bounded: N*2)"/]

    subgraph FetchWorkers["N Fetch Workers"]
        fw["FetchTask::fetch()<br/>- check cache<br/>- fetch_block_hash()<br/>- fetch_block()<br/>- retry with backoff"]
    end

    fetch_to_compute_ch[/"fetch_to_compute channel<br/>(bounded: M*2)"/]

    subgraph ComputeWorkers["M Compute Workers"]
        cw["ComputeTask::work()<br/>1. ExtractBlockData<br/>2. Verify (hash chain)<br/>3. FinalVerify (boundaries)"]
    end

    compute_loop[/"compute_to_compute channel<br/>(unbounded, recursive)"/]

    final_ch[/"final_jobs channel<br/>(bounded: M*2)"/]

    subgraph Collector["Main Thread Collector"]
        collector["Collect FinalVerify tasks<br/>Execute final cross-batch verify<br/>set_highest_verified_block()<br/>read_blocks_from_cache()"]
    end

    subgraph Storage["FetchStorage Backend"]
        direction TB
        inmem["InMemory<br/>HashMap&lt;Vec&lt;u8&gt;, RawBlockData&gt;"]
        redb_be["ReDB<br/>(redb_backend.rs)"]
        pg_be["PostgreSQL<br/>(postgres_backend.rs)"]
    end

    subgraph RuntimeDispatch["Runtime Version Dispatch"]
        direction TB
        mnsv["Block digest: MNSV header"]
        rt_enum["RuntimeVersion enum<br/>V0_17_0 .. V0_22_0"]
        metadata["MidnightMetadata trait<br/>process_block_with_protocol&lt;M&gt;()"]
        mnsv --> rt_enum --> metadata
    end

    pusher --> fetch_job_ch --> fw
    fw -->|"ComputeTask::ExtractBlockData"| fetch_to_compute_ch --> cw
    cw -->|"Verify / recursive"| compute_loop --> cw
    cw -->|"FinalVerify"| final_ch --> collector

    fw <-->|"get/insert block_data"| Storage
    cw <-->|"get/insert block_data"| Storage
    cw --> RuntimeDispatch

    client_rpc["MidnightNodeClient<br/>(RPC :9944)"]
    fw -->|"WebSocket"| client_rpc

    style JobPusher fill:#e3f2fd,stroke:#1565c0,color:#000
    style FetchWorkers fill:#e3f2fd,stroke:#1565c0,color:#000
    style ComputeWorkers fill:#e3f2fd,stroke:#1565c0,color:#000
    style Collector fill:#e3f2fd,stroke:#1565c0,color:#000
    style Storage fill:#e8eaf6,stroke:#283593,color:#000
    style RuntimeDispatch fill:#fce4ec,stroke:#c62828,color:#000
```

### Pipeline stages

1. **Job Pusher** -- iterates `min_height..max_height` in steps of `BLOCKS_PER_JOB` (100), pushes `FetchTask::FetchBlocks { min, max }` onto the bounded `fetch_job` channel.
2. **Fetch Workers** (N) -- each connects its own `MidnightNodeClient`. Pulls from `fetch_job`, checks storage cache, fetches missing blocks via RPC with exponential backoff, emits `ComputeTask::ExtractBlockData`.
3. **Compute Workers** (M) -- pull from both `fetch_to_compute` and `compute_to_compute` (biased toward fetch). Execute three phases:
   - `ExtractBlockData` -- calls `extract_data()` which dispatches on `RuntimeVersion` via `MidnightMetadata` trait, stores `RawBlockData` in storage. Emits `Verify`.
   - `Verify` -- checks parent-child hash chain within the batch. Emits `FinalVerify`.
   - `FinalVerify` -- checks cross-batch boundary hashes. Sent to main thread.
4. **Collector** -- main thread gathers all `FinalVerify` tasks, executes them, calls `set_highest_verified_block()`, then `read_blocks_from_cache()` to return all blocks.

---

## Diagram 3: Ledger Version Dispatch

```mermaid
graph TB
    subgraph Input["Input"]
        src_tx["SourceTransactions<br/>{blocks: Vec&lt;RawBlockData&gt;, network_id}"]
    end

    subgraph ContextBuild["Context Construction"]
        build_raw["build_fork_aware_context_raw()"]
        init["ForkAwareLedgerContext::new_from_wallet_seeds(<br/>initial_version, network_id, seeds)"]
        replay["for block in blocks {<br/>  ctx = ctx.update_from_block(block)<br/>}"]
        build_raw --> init --> replay
    end

    subgraph ForkCtx["ForkAwareLedgerContext (enum)"]
        direction LR
        l7["Ledger7(<br/>ledger_7::LedgerContext&lt;Db7&gt;)"]
        l8["Ledger8(<br/>ledger_8::LedgerContext&lt;Db8&gt;)"]
    end

    subgraph AutoFork["Hard Fork Auto-Transition"]
        check{"block.ledger_version()<br/>!= ctx.version()?"}
        next_fork["next_fork()<br/>fork_context_7_to_8()"]
        check -->|"Ledger7 ctx + Ledger8 block"| next_fork
        next_fork --> l8
    end

    subgraph Dispatch["Builder::to_versioned_builder()"]
        dispatch_fn["ctx.dispatch(f7, f8)"]

        subgraph V7Path["v7 Path (builders/ledger_7.rs)"]
            v7_alias["#path = 'common'<br/>use ledger_7 as ledger_helpers_local"]
            v7_builders["BatchesBuilder<br/>ContractDeployBuilder<br/>ContractCallBuilder<br/>ContractMaintenanceBuilder<br/>ClaimRewardsBuilder<br/>SingleTxBuilder<br/>RegisterDustAddressBuilder<br/>DeregisterDustAddressBuilder"]
            v7_prover["LocalProofServer only"]
            v7_limits["No ContractCustom<br/>No Remote Prover"]
        end

        subgraph V8Path["v8 Path (builders/ledger_8.rs)"]
            v8_alias["#path = 'common'<br/>use ledger_8 as ledger_helpers_local"]
            v8_builders["All v7 builders +<br/>CustomContractBuilder"]
            v8_prover["LocalProofServer<br/>RemoteProofServer"]
        end

        dispatch_fn -->|"Ledger7"| V7Path
        dispatch_fn -->|"Ledger8"| V8Path
    end

    subgraph DualCompile["Dual Compilation Pattern"]
        direction TB
        common_dir["builders/common/<br/>batches.rs, claim_rewards.rs,<br/>contract_call.rs, contract_deploy.rs,<br/>contract_maintenance.rs, contract_custom.rs,<br/>register_dust_address.rs, deregister_dust_address.rs,<br/>single_tx.rs, do_nothing.rs,<br/>build_txs_ext.rs, transactions.rs,<br/>tx_serialization.rs, type_convert.rs"]
        compile_note["Each .rs file is compiled TWICE:<br/>once with ledger_7 types, once with ledger_8 types.<br/>The alias ledger_helpers_local resolves to<br/>the correct ledger version's types."]
        common_dir --- compile_note
    end

    src_tx --> build_raw
    replay --> ForkCtx
    ForkCtx --> AutoFork
    ForkCtx --> Dispatch
    V7Path -.->|"#path"| DualCompile
    V8Path -.->|"#path"| DualCompile

    style V7Path fill:#efebe9,stroke:#795548,color:#000
    style V8Path fill:#efebe9,stroke:#795548,color:#000
    style AutoFork fill:#fff8e1,stroke:#f9a825,color:#000
    style DualCompile fill:#f3e5f5,stroke:#7b1fa2,color:#000
    style ForkCtx fill:#e8f5e9,stroke:#2e7d32,color:#000
    style v7_limits fill:#ffebee,stroke:#c62828,color:#000
```

### How dual compilation works

Both `builders/ledger_7.rs` and `builders/ledger_8.rs` use the same pattern:

```rust
#[path = "common"]
#[allow(clippy::duplicate_mod)]
pub mod inner {
    pub use midnight_node_ledger_helpers::ledger_N as ledger_helpers_local;
    // ... mod declarations for each builder
}
pub use inner::*;
```

The `common/*.rs` files reference `ledger_helpers_local` for all ledger types (e.g., `Wallet`, `LedgerContext`, `ProofProvider`). Because `ledger_helpers_local` is aliased to the version-specific module before the `mod` declarations, each file compiles against the correct ledger version's types.

The same pattern is used in `commands/fork/ledger_7.rs` and `commands/fork/ledger_8.rs` for read-only commands (show-wallet, dust-balance, etc.).

### v7 limitations

- **No `ContractCustom`** -- `EncodedOutputInfo` does not implement ledger_7 `BuildOutput`
- **No Remote Prover** -- returns `BuilderConstructionError::RemoteProverNotSupportedForLedger7`
- **Auto-transition** -- `update_from_block()` detects when a Ledger7 context receives a Ledger8 block and calls `next_fork()` -> `fork_context_7_to_8()` to migrate state
