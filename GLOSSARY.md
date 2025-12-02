# Midnight Node Glossary

This glossary defines domain-specific terms and jargon used throughout the Midnight Node codebase. Terms are organized by category for easier navigation.

---

## Midnight-Specific Terms

### [Compact](https://docs.midnight.network/learn/glossary#compact)

A domain-specific programming language designed by Midnight for writing privacy-preserving smart contracts. Compact is TypeScript-like in syntax and compiles to zero-knowledge circuits. Contracts written in Compact can protect sensitive data while still enabling on-chain verification.

### [cNIGHT](https://docs.midnight.network/about-dust)

A wrapped representation of NIGHT tokens on the Cardano blockchain. cNIGHT tokens can be observed by the Midnight node to generate DUST tokens, enabling cross-chain token economics between Cardano and Midnight.

### [DUST](https://docs.midnight.network/about-dust)

The native utility token of the Midnight network, used for:
- Paying transaction fees (gas)
- Block rewards for validators
- Staking and governance participation

DUST can be generated from cNIGHT holdings observed on Cardano through the "cNIGHT generates DUST" mechanism.

### [Glacier Drop](https://www.midnight.gd)

A token distribution mechanism allowing eligible participants to claim DUST tokens. The Glacier Drop uses Cardano-based redemption contracts that are observed by the Midnight node to credit tokens to users.

### [Midnight Ledger](https://docs.midnight.network/learn/glossary#ledger)

The core state machine managing Midnight's privacy-preserving transactions. The ledger uses the Halo2 proving system and maintains both public blockchain state and private user state through zero-knowledge proofs.

### [NIGHT](https://docs.midnight.network/learn/what-is-midnight/midnight-combined-model#the-night-token-and-dust-resource)

The primary token of the Midnight ecosystem. NIGHT exists on both Midnight (as DUST) and Cardano (as cNIGHT), with bridge mechanisms enabling cross-chain functionality.

### [Shielded Transaction](https://docs.midnight.network/learn/glossary#shielded)

A transaction where the amounts, participants, or other details are hidden using zero-knowledge proofs. Only the validity of the transaction is publicly verifiable, not its contents.

### [tDUST](https://docs.midnight.network/develop/how-to/acquire-tokens)

Test DUST tokens used on Midnight testnets (devnet, testnet). tDUST has no monetary value and is obtained from faucets for development and testing purposes.

### [Unshielded Transaction](https://docs.midnight.network/develop/how-midnight-works/keeping-data-private)

A transaction where amounts and participants are publicly visible on the blockchain, similar to traditional blockchain transactions. Midnight supports both shielded and unshielded transactions.

### [ZSwap](https://docs.midnight.network/learn/glossary#zswap)

The zero-knowledge swap/transfer protocol used by Midnight for privacy-preserving token transfers. ZSwap manages UTXOs (Unspent Transaction Outputs) with cryptographic commitments, enabling private value transfers while maintaining verifiable correctness.

---

## Substrate/Polkadot Terms

### [AURA (Authority Round)](https://docs.substrate.io/reference/glossary/#aura)

A slot-based block production consensus mechanism used by Midnight. Validators take turns producing blocks in a round-robin fashion based on time slots. AURA provides predictable block times.

### [BEEFY (Bridge Efficiency Enabling Finality Yielder)](https://wiki.polkadot.network/docs/learn-beefy)

A secondary finality protocol that produces compact finality proofs suitable for bridging to external chains. Midnight uses BEEFY for cross-chain communication with Cardano.

### [Dispatchable](https://docs.substrate.io/reference/glossary/#dispatch)

A function in a Substrate pallet that can be called from outside the runtime, typically through an extrinsic. Dispatchables define the public interface of a pallet.

### [Extrinsic](https://docs.substrate.io/reference/glossary/#extrinsic)

Any data that originates outside the runtime and is included in a block. Extrinsics include:
- **Signed extrinsics**: Transactions signed by an account
- **Unsigned extrinsics**: Data without a signature (e.g., inherents)

### [FRAME (Framework for Runtime Aggregation of Modularized Entities)](https://docs.substrate.io/reference/glossary/#frame)

Substrate's modular framework for building blockchain runtimes. FRAME provides a library of pallets that can be composed to create custom blockchain logic.

### [GRANDPA (GHOST-based Recursive ANcestor Deriving Prefix Agreement)](https://docs.substrate.io/reference/glossary/#grandpa)

A Byzantine fault-tolerant finality gadget that provides deterministic finality for blocks. Once GRANDPA finalizes a block, it cannot be reverted.

### [Hook](https://docs.substrate.io/build/custom-pallets/#hooks)

Lifecycle callbacks in a Substrate pallet that execute at specific points in block processing:
- `on_initialize`: Runs at the start of each block
- `on_finalize`: Runs at the end of each block
- `on_runtime_upgrade`: Runs during runtime upgrades

### [Inherent](https://docs.substrate.io/reference/glossary/#inherent)

A special type of unsigned extrinsic that provides data the runtime needs but cannot verify independently, such as timestamps or external observations. Midnight uses inherents to inject Cardano observation data.

### [MMR (Merkle Mountain Range)](https://docs.substrate.io/reference/glossary/#merkle-mountain-range-mmr)

A data structure used for efficient light client proofs and cross-chain verification. Midnight uses MMR with BEEFY for bridge operations.

### [Origin](https://docs.substrate.io/reference/glossary/#origin)

The source of a dispatchable call in Substrate. Common origins include:
- `Root`: Highest privilege level (sudo)
- `Signed`: A regular account
- `None`: Unsigned/inherent calls

### [Pallet](https://docs.substrate.io/reference/glossary/#pallet)

A modular component in a Substrate runtime that encapsulates specific blockchain functionality. Pallets contain storage, events, errors, and dispatchable functions.

### [Runtime](https://docs.substrate.io/reference/glossary/#runtime)

The state transition function of a Substrate blockchain. The runtime defines all business logic and is compiled to WebAssembly (WASM) for execution. Runtime upgrades can be performed without hard forks.

### [Storage](https://docs.substrate.io/build/runtime-storage/)

On-chain state managed by pallets. Substrate provides various storage types:
- `StorageValue`: Single value
- `StorageMap`: Key-value mapping
- `StorageDoubleMap`: Two-key mapping

### [Weight](https://docs.substrate.io/reference/glossary/#weight)

A measure of computational resources consumed by an operation in Substrate. Weights are used to calculate transaction fees and prevent denial-of-service attacks.

---

## Cardano Terms

### [db-sync](https://docs.cardano.org/cardano-components/cardano-db-sync/about-db-sync/)

A component that follows the Cardano blockchain and stores data in a PostgreSQL database. Midnight nodes query db-sync to observe Cardano state for cNIGHT tokens and governance data.

### [Main Chain](https://docs.midnight.network/learn/what-is-midnight/midnight-combined-model)

In the context of Midnight, refers to the Cardano blockchain. Midnight operates as a "partner chain" to Cardano, observing mainchain state for cross-chain functionality.

### McBlockHash / McTxHash

Main chain (Cardano) block hash and transaction hash types. Used to track positions and references to Cardano blockchain data.

### [Partner Chain](https://docs.midnight.network/learn/what-is-midnight/midnight-combined-model)

A blockchain that operates alongside Cardano, leveraging its security and infrastructure while providing specialized functionality. Midnight is a partner chain focused on data protection.

### [Policy ID](https://docs.cardano.org/native-tokens/learn/)

A 28-byte identifier on Cardano that uniquely identifies a native asset's minting policy. Used to identify cNIGHT tokens and governance assets.

### [Reward Address](https://docs.cardano.org/learn/stake-pool-operators/staking/)

A Cardano address used for staking rewards. In Midnight, reward addresses are used to map Cardano wallets to DUST public keys for the cNIGHT-to-DUST mechanism.

---

## Cryptographic Terms

### [Circuit](https://docs.midnight.network/learn/glossary#zero-knowledge-zk-circuit)

In zero-knowledge cryptography, a representation of a computation as a series of gates and wires. Compact contracts compile to circuits that can be proven and verified.

### [Commitment](https://docs.midnight.network/learn/glossary#pedersen-commitments)

A cryptographic scheme that allows one to commit to a value while keeping it hidden, with the ability to reveal it later. Used in ZSwap for hiding transaction amounts.

### [Halo2](https://docs.midnight.network/learn/glossary#halo-2)

A zero-knowledge proving system used by Midnight's ledger. Halo2 enables efficient proof generation and verification without a trusted setup.

### KZG Commitment

A polynomial commitment scheme named after Kate, Zaverucha, and Goldberg. KZG commitments allow a prover to commit to a polynomial and later prove evaluations of that polynomial at specific points. Midnight uses KZG commitments with the PLONK proof system for efficient zero-knowledge proofs.

### [Merkle Tree](https://docs.midnight.network/learn/glossary#merkle-tree)

A tree data structure where each leaf node is a hash of data, and each non-leaf node is a hash of its children. Used for efficient verification of data integrity.

### PLONK

A zero-knowledge proof system whose name derives from "Permutations over Lagrange-bases for Oecumenical Noninteractive arguments of Knowledge". Midnight uses a PLONK-based proof system with KZG polynomial commitments for efficient proof generation and verification.

### [Prover](https://docs.midnight.network/learn/glossary#prover)

In zero-knowledge systems, the entity that generates a proof to convince a verifier of a statement's truth without revealing the underlying data.

### [SNARK (Succinct Non-interactive ARgument of Knowledge)](https://docs.midnight.network/learn/glossary#zk-snarks)

A type of zero-knowledge proof that is:
- **Succinct**: Proofs are small and quick to verify
- **Non-interactive**: No back-and-forth communication required
- **Argument of Knowledge**: Proves knowledge of secret data

### [Verifier](https://docs.midnight.network/learn/glossary#verifier)

In zero-knowledge systems, the entity that checks the validity of a proof without learning the secret information.

### [Witness](https://docs.midnight.network/learn/glossary#witness)

Private inputs to a zero-knowledge circuit that are used to generate a proof but are not revealed publicly. In Compact contracts, witnesses provide secret data for computations.

### [Zero-Knowledge Proof (ZKP)](https://docs.midnight.network/learn/glossary#zero-knowledge-proof-zkp)

A cryptographic method allowing one party (the prover) to prove to another (the verifier) that a statement is true without revealing any information beyond the statement's validity.

---

## Governance Terms

### Council

A governance body in Midnight responsible for collective decision-making. Council membership is observed from Cardano and synchronized to the Midnight chain.

### Federated Authority

A governance mechanism requiring approval from multiple independent authority bodies before executing privileged operations. Ensures no single body can unilaterally make critical decisions.

### Motion

A proposed action in the federated authority system. Motions must receive approvals from all required governance bodies before execution.

### Technical Committee

A governance body with specialized technical oversight responsibilities. Like the Council, its membership is observed from Cardano.

---

## Transaction & State Terms

### [Block Context](https://docs.substrate.io/build/custom-pallets/)

Runtime information about the current block being processed, including:
- Parent block hash
- Current timestamp
- Block number

### [CardanoPosition](https://docs.midnight.network/learn/what-is-midnight/midnight-combined-model)

A data structure tracking the sync position on Cardano:
- Block hash
- Block number
- Transaction index within block

Used to ensure deterministic processing of Cardano observations.

### [CMST (Cardano Midnight System Transaction)](https://docs.midnight.network/learn/what-is-midnight/midnight-combined-model)

A system-level transaction generated from Cardano observations. CMSTs update Midnight state based on events observed on Cardano (registrations, token movements, etc.).

### Gas Cost

The computational cost of executing a transaction, measured in gas units. Used alongside storage costs to calculate total transaction fees.

### [State Root / State Key](https://docs.midnight.network/learn/glossary#state)

A cryptographic hash representing the current state of the Midnight ledger. Used to verify state integrity and track state transitions.

### Storage Cost

The cost of storing data on-chain, measured separately from computational (gas) costs.

### [System Transaction](https://docs.midnight.network/learn/what-is-midnight/midnight-combined-model)

A privileged transaction type that can modify state without user signatures. Used for applying Cardano-observed state changes to the Midnight ledger.

### [UTXO (Unspent Transaction Output)](https://docs.midnight.network/learn/glossary#qualified-coin)

A model for tracking token ownership where each "output" of a transaction can only be spent once. ZSwap uses a UTXO model with cryptographic commitments for privacy.

---

## Network & Environment Terms

### [Chain Spec / Chain Specification](https://docs.substrate.io/build/chain-spec/)

A JSON configuration file defining the genesis state and parameters of a blockchain network. Includes initial balances, authorities, and runtime configuration.

### [Devnet](https://docs.midnight.network/learn/glossary#devnet)

A development network for early-stage testing. Less stable than testnet, used for active development.

### [Genesis](https://docs.substrate.io/build/genesis-configuration/)

The initial state of a blockchain at block 0. Includes initial account balances, authority configurations, and runtime parameters.

### [Mainnet](https://docs.midnight.network/learn/glossary#mainnet)

The production network where real value transactions occur. Midnight mainnet is the live deployment with actual NIGHT/DUST tokens.

### QAnet

A quality assurance network used for integration testing before testnet deployment.

### [Testnet](https://docs.midnight.network/learn/glossary#testnet)

A public test network that simulates mainnet conditions. Uses test tokens (tDUST) with no real value.

---

## Development Terms

### [Benchmarking](https://docs.substrate.io/test/benchmark/)

The process of measuring the computational cost of runtime operations to determine accurate weights for transaction fees.

### [Host Function](https://docs.substrate.io/reference/glossary/#host-function)

A function provided by the native (non-WASM) side of Substrate that can be called from within the WASM runtime. Used for operations that cannot be performed in WASM, like ledger storage access.

### [Runtime API](https://docs.substrate.io/reference/glossary/#runtime-api)

An interface defined by the runtime that allows the node client to query runtime state or perform computations. Exposed via RPC to external clients.

### [Try-Runtime](https://docs.substrate.io/reference/command-line-tools/try-runtime/)

A Substrate tool for testing runtime upgrades by simulating them against live chain state without actually deploying.

---

## See Also

- [pallets/](pallets/README.md) - Pallet package index
- [Midnight Documentation](https://docs.midnight.network) - Official Midnight documentation
- [Substrate Documentation](https://docs.substrate.io) - Substrate framework documentation
- [Polkadot Wiki](https://wiki.polkadot.network) - Polkadot ecosystem documentation

