# Midnight Node Codebase Review Report

**Date:** December 3, 2025  
**Reviewer:** AI Assistant  
**Scope:** Quality, Test Coverage, Substrate Best Practice Compliance

---

## Executive Summary

The `midnight-node` codebase demonstrates **solid Substrate/FRAME architecture** with well-organized workspace structure, comprehensive pallet implementations, and good test coverage. The project follows many Substrate best practices while integrating unique cross-chain functionality with Cardano. This review identifies areas of strength and opportunities for improvement.

### Overall Assessment: **Good** (with minor improvements recommended)

| Category | Rating | Notes |
|----------|--------|-------|
| **Architecture** | ✅ Good | Well-organized workspace, clear module separation |
| **Test Coverage** | ✅ Good | 134 test functions, E2E tests, benchmarks present |
| **Substrate Compliance** | ✅ Good | Follows FRAME conventions, proper storage patterns |
| **Rust Quality** | ⚠️ Fair | Some `.expect()` usage in production paths |
| **Documentation** | ⚠️ Fair | Good READMEs, sparse in-code documentation |
| **Security** | ✅ Good | Proper authorization patterns, minimal unsafe code |

---

## 1. Workspace Structure & Architecture

### Strengths

1. **Clear Modular Design**
   - Well-separated concerns: `pallets/`, `primitives/`, `runtime/`, `node/`, `ledger/`
   - Each module has dedicated `README.md` files with architecture diagrams
   - Consistent use of workspace dependencies in root `Cargo.toml`

2. **Workspace Organization**
   ```
   midnight-node/
   ├── pallets/           # Core FRAME pallets (6 pallets)
   ├── primitives/        # Shared types and traits
   ├── runtime/           # Runtime composition
   ├── node/              # Node executable
   ├── ledger/            # Host function bridge
   ├── tests/             # E2E and integration tests
   └── util/              # Development utilities
   ```

3. **Dependency Management**
   - Centralized dependency versions in workspace `Cargo.toml`
   - Uses Polkadot SDK `polkadot-stable2509` tag for stability
   - External dependencies properly pinned via git tags

### Areas for Improvement

1. **Workspace Lints Configuration**
   - Current Clippy configuration is very permissive (most lints set to `allow`)
   - Only `correctness` and `complexity` set to `warn`
   
   **Recommendation:** Consider enabling more lints incrementally:
   ```toml
   [workspace.lints.clippy]
   pedantic = { level = "warn", priority = 1 }
   suspicious = { level = "warn", priority = 1 }
   ```

---

## 2. Pallet Review

### 2.1 `pallet-midnight` (Core)

**Location:** `pallets/midnight/src/lib.rs`

**Strengths:**
- Proper storage versioning (`STORAGE_VERSION`)
- Implements `ValidateUnsigned` for transaction pool validation
- Well-structured `on_initialize` and `on_finalize` hooks
- Proper use of `DispatchClass::Operational` for privileged calls
- Benchmarking support enabled

**Concerns:**
1. **Heavy use of `.expect()` in production paths:**
   ```rust
   let state_key = StateKey::<T>::get().expect("Failed to get state key");
   ```
   These could panic in production. Consider:
   ```rust
   let state_key = StateKey::<T>::get().ok_or(Error::<T>::StateKeyNotFound)?;
   ```

2. **Weight estimation comment:**
   ```rust
   // TODO: Benchmark Weight in case of a real hard-fork
   ```
   This should be addressed before mainnet.

### 2.2 `pallet-federated-authority`

**Location:** `pallets/federated-authority/src/lib.rs`

**Strengths:**
- Well-designed federated governance pattern
- Proper use of `BoundedBTreeSet` for approvals
- Clear motion lifecycle (propose → approve → close)
- Root origin dispatch for approved motions
- Comprehensive benchmarking and weight calculation

**Test Coverage:** Excellent - 25+ test cases covering:
- Motion approval/revocation flows
- Edge cases (max authorities, expiration)
- Authorization checks
- End-to-end governance flow

### 2.3 `pallet-federated-authority-observation`

**Location:** `pallets/federated-authority-observation/src/lib.rs`

**Strengths:**
- Implements `ProvideInherent` correctly for cross-chain observation
- Proper genesis config with `#[derive(frame_support::DefaultNoBound)]`
- Good separation of Council vs Technical Committee

**Concerns:**
1. **Genesis build uses `.expect()`:**
   ```rust
   .try_into().expect("Council mainchain members exceeds max members")
   ```
   Genesis panics are acceptable but should be documented.

### 2.4 `pallet-cnight-observation`

**Location:** `pallets/cnight-observation/src/lib.rs`

**Strengths:**
- Complex cross-chain token observation well-implemented
- Proper inherent extrinsic pattern
- Event emission for all significant state changes

**Concerns:**
1. **Weight set to zero for mandatory inherent:**
   ```rust
   #[pallet::weight((0, DispatchClass::Mandatory))]
   pub fn process_tokens(...)
   ```
   Mandatory inherents should still have accurate weights.

2. **TODO comment indicates incomplete design:**
   ```rust
   // TODO: Read from ledger state directly ?
   pub type UtxoOwners<T: Config> = StorageMap<...>;
   ```

### 2.5 `pallet-midnight-system`

**Location:** `pallets/midnight-system/src/lib.rs`

**Concerns:**
1. **No test coverage** - Only file is `lib.rs` with no test module
2. **Missing documentation** - No module-level doc comments

**Recommendation:** Add unit tests and documentation.

---

## 3. Test Coverage Analysis

### Test Metrics

| Metric | Value |
|--------|-------|
| Test files with `#[test]` | ~15 files |
| Total test functions | 134 |
| E2E test scenarios | 6 |
| Benchmarked pallets | 5 |

### Test Distribution

| Pallet | Unit Tests | Benchmarks |
|--------|------------|------------|
| `pallet-midnight` | ✅ Yes (12+) | ✅ Yes |
| `pallet-federated-authority` | ✅ Yes (25+) | ✅ Yes |
| `pallet-federated-authority-observation` | ✅ Yes (30+) | ✅ Yes |
| `pallet-cnight-observation` | ✅ Yes (15+) | ❌ No |
| `pallet-midnight-system` | ❌ No | ❌ No |
| `pallet-version` | ✅ Yes (1) | ❌ Partial |
| Runtime | ✅ Yes (5+) | ✅ Yes |

### E2E Tests (tests/e2e/)

Strong integration test coverage:
- `register_for_dust_production` - cNIGHT registration flow
- `deploy_governance_contracts_and_validate_membership_reset` - Governance
- `register_2_cardano_same_dust_address_production` - Multi-registration
- `cnight_produces_dust` - Token minting validation
- `deregister_from_dust_production` - Deregistration flow
- `alice_cannot_deregister_bob` - Authorization test

### Gaps to Address

1. **`pallet-midnight-system`** - Zero test coverage
2. **`pallet-cnight-observation`** - Missing benchmarks
3. **Some tests are `#[ignore]`:**
   ```rust
   #[ignore = "TODO COST MODEL - fix when new Ledger's cost model is available"]
   #[ignore = "TODO UNSHIELDED - fix when Claim Mint is properly handled"]
   ```

---

## 4. Substrate Best Practices Compliance

### ✅ Compliant Patterns

1. **Storage Versioning**
   All pallets properly define storage versions:
   ```rust
   const STORAGE_VERSION: StorageVersion = StorageVersion::new(0);
   ```

2. **Proper Origin Checks**
   - `ensure_root()` for privileged operations
   - `ensure_none()` for inherent extrinsics
   - `ensure_signed()` for user operations

3. **Weight System**
   - Benchmarks provide weight functions
   - `DispatchClass::Mandatory` for inherents
   - `DispatchClass::Operational` for governance calls

4. **Event Emission**
   All pallets use `#[pallet::generate_deposit]` pattern:
   ```rust
   #[pallet::event]
   #[pallet::generate_deposit(pub(super) fn deposit_event)]
   pub enum Event<T: Config> { ... }
   ```

5. **Bounded Storage Types**
   Proper use of `BoundedVec`, `BoundedBTreeSet`, `BoundedBTreeMap`

6. **Genesis Configuration**
   Pallets that need genesis state implement `BuildGenesisConfig`

### ⚠️ Areas for Improvement

1. **Documentation Comments**
   - Storage items often lack `#[doc = "..."]` annotations
   - Error variants could have more descriptive documentation

2. **Runtime Migrations**
   - `migrations::IncrementSudoSufficients<Runtime>` exists
   - Should document migration history in `CHANGELOG.md`

3. **Try-Runtime Support**
   - Configured at runtime level
   - Pallets should implement `pre_upgrade` / `post_upgrade` hooks

---

## 5. Rust Best Practices

### Strengths

1. **Type System Usage**
   - Good use of newtypes (`DustPublicKeyBytes`, `CardanoRewardAddressBytes`)
   - Proper trait bounds on generics
   - `PhantomData` used correctly

2. **Error Handling in Tests**
   - Tests use `assert_ok!` and `assert_noop!` macros correctly
   - Good coverage of error conditions

3. **Code Organization**
   - Clear module structure
   - Related types grouped together

### Concerns

1. **`.expect()` in Non-Genesis Code**
   Found 118 instances of `unwrap()` or `expect()`:
   - ~50% are in test code (acceptable)
   - ~30% are in production paths (concerning)
   - ~20% are in genesis/initialization (acceptable)
   
   **High-priority refactoring targets:**
   ```rust
   // pallets/midnight/src/lib.rs
   let state_key = StateKey::<T>::get().expect("Failed to get state key");
   ```

2. **TODO/FIXME Comments**
   Found 39 TODO/FIXME comments indicating incomplete features:
   - Cost model work pending
   - Unshielded token handling incomplete
   - Some benchmarking TODO items

3. **Formatting Compliance**
   - `rustfmt.toml` properly configured
   - Uses hard tabs and 100 character line width
   - Consider enforcing via CI (appears to be via Earthfile)

---

## 6. Security Review

### ✅ Secure Patterns

1. **Authorization**
   - Root-only operations properly gated
   - Inherent extrinsics use `ensure_none()`
   - Cross-chain observation uses inherent pattern (block author controlled)

2. **Unsafe Code**
   - Minimal unsafe usage (only in `runtime/build.rs` for env vars)
   - No unsafe in pallet business logic

3. **Transaction Validation**
   - `ValidateUnsigned` properly implements `pre_dispatch`
   - Block context validation prevents timing attacks

4. **Dependency Auditing**
   - `deny.toml` configured with:
     - License checking (Apache-2.0, MIT, etc.)
     - Advisory database integration
     - Known vulnerability ignores documented with reasons
     - Source repository allowlist

### ⚠️ Areas for Review

1. **Trusted Input Assumptions**
   Some genesis configuration uses `.expect()` on input validation:
   ```rust
   .expect("Council mainchain members exceeds max members")
   ```
   These are acceptable for genesis but could be errors during runtime upgrades.

2. **Cross-Chain Trust Model**
   - Relies on `ProvideInherent` for Cardano observations
   - Block author controls inherent data
   - This is documented design but worth security review

---

## 7. Documentation Assessment

### ✅ Good Documentation

1. **README Files**
   - Root README with architecture overview
   - Each major module has README.md
   - ASCII diagrams for architecture visualization

2. **Glossary**
   - `GLOSSARY.md` defines domain-specific terms
   - Cross-referenced in documentation

3. **Chain Specifications**
   - Well-documented chain spec generation
   - Network-specific configurations explained

### ⚠️ Documentation Gaps

1. **In-Code Documentation**
   - Module-level `//!` comments exist but sparse
   - Function documentation is inconsistent
   - Storage items often undocumented

2. **API Documentation**
   - RPC methods documented in README
   - Could use more detailed parameter descriptions

3. **Architecture Decision Records**
   - No ADR directory found
   - Consider documenting design decisions

---

## 8. Recommendations

### High Priority

1. **Replace `.expect()` with proper error handling in pallet code**
   ```rust
   // Before
   StateKey::<T>::get().expect("Failed to get state key")
   // After  
   StateKey::<T>::get().ok_or(Error::<T>::StateKeyNotInitialized)?
   ```

2. **Add tests for `pallet-midnight-system`**

3. **Add benchmarks for `pallet-cnight-observation`**

4. **Address ignored tests**
   - Investigate and resolve or document blockers

### Medium Priority

5. **Enhance Clippy configuration**
   - Gradually enable more lints
   - Add `clippy::unwrap_used` to catch runtime panics

6. **Add documentation to storage items**
   ```rust
   #[pallet::storage]
   /// Stores the current ledger state root key.
   /// Initialized during genesis and updated after each block.
   pub type StateKey<T: Config> = StorageValue<_, BoundedVec<u8, StateKeyLimit>>;
   ```

7. **Create Architecture Decision Records**
   - Document cross-chain trust model
   - Document consensus design choices

### Low Priority

8. **Consolidate TODO comments**
   - Create tracking issues for TODO items
   - Remove resolved TODOs

9. **Add integration tests for runtime upgrades**
   - Test migration scenarios
   - Verify `try-runtime` compatibility

---

## 9. Conclusion

The `midnight-node` codebase demonstrates **professional quality** Substrate development with good adherence to framework conventions. The main areas for improvement are:

1. **Error handling** - Reduce `.expect()` usage in production paths
2. **Test coverage** - Add tests for `pallet-midnight-system`  
3. **Documentation** - Enhance in-code documentation

The architecture is sound, security patterns are appropriate for a blockchain node, and the cross-chain integration with Cardano is well-designed. With the recommended improvements, this codebase would meet production-ready standards.

---

## 10. Primitives Review

**Location:** `primitives/`

### Structure

| Module | Purpose | Test Coverage |
|--------|---------|---------------|
| `midnight/` | Core traits (`LedgerStateProviderMut`, `LedgerBlockContextProvider`, `MidnightSystemTransactionExecutor`) | ❌ No tests |
| `ledger/` | Metrics and externality extensions | ❌ No tests |
| `cnight-observation/` | Cardano UTXO observation types (`CardanoPosition`, `ObservedUtxo`) | ❌ No tests |
| `federated-authority-observation/` | Governance observation types | ❌ No tests |
| `mainchain-follower/` | Data source interface types, db-sync queries | ❌ No tests |

### Strengths

1. **Clear Trait Definitions**
   - `LedgerStateProviderMut` provides clean abstraction for ledger state mutation
   - `LedgerBlockContextProvider` separates block context from pallet logic
   - `MidnightSystemTransactionExecutor` enables system transaction execution

2. **no_std Compatibility**
   - All primitives use `#![cfg_attr(not(feature = "std"), no_std)]`
   - Enables runtime WASM compilation

3. **Well-Defined Types**
   - `TransactionType` and `TransactionTypeV2` for transaction categorization
   - `well_known_keys` module for storage key constants

### Concerns

1. **No Unit Tests** - Primitives modules have no test coverage
2. **TODO Comment in mainchain-follower:**
   ```rust
   // TODO: Change the error type to something explicit
   async fn get_utxos_up_to_capacity(...) -> Result<..., Box<dyn std::error::Error + Send + Sync>>
   ```

### Recommendations

- Add unit tests for type serialization/deserialization
- Replace `Box<dyn Error>` with explicit error types

---

## 11. Ledger Review

**Location:** `ledger/`

### Structure

| Module | Purpose | Test Coverage |
|--------|---------|---------------|
| `src/lib.rs` | Module-parameterized ledger bridge | ❌ No tests |
| `src/storage.rs` | ParityDB storage management | ✅ Tests (3) |
| `src/host_api/` | Host function implementations | ❌ No tests |
| `src/json.rs` | JSON serialization utilities | ✅ Tests (2) |
| `src/utils.rs` | Utility functions | ✅ Tests (1) |
| `helpers/` | Transaction building, wallet management | ✅ Tests (5+) |

### Strengths

1. **Module Parameterization Pattern**
   - Clever use of Rust's module system for hard-fork support
   - `hard_fork_test` and `latest` versions share common code
   - Documented with link to pattern explanation

2. **Clean Type Re-exports**
   ```rust
   pub mod types {
       #[cfg(hardfork_test)]
       pub use super::hard_fork_test::types as active_version;
       #[cfg(not(hardfork_test))]
       pub use super::latest::types as active_version;
   }
   ```

3. **Helper Library**
   - Comprehensive transaction builder patterns
   - Wallet management (HD, shielded, unshielded)
   - Contract deployment and interaction helpers

### Concerns

1. **Host API Lacks Tests** - Critical host functions untested
2. **Multiple TODO Comments:**
   ```rust
   // TODO COST MODEL: Needs to be redone with the new ledger cost model
   // TODO: is this rehash necessary?
   ```

3. **Unsafe Storage Operations:**
   ```rust
   unsafe_drop_default_storage::<ParityDb>();
   ```
   Used for test cleanup - acceptable but worth noting.

### Recommendations

- Add tests for host API functions
- Address cost model TODOs before mainnet
- Document unsafe usage rationale

---

## 12. Node Review

**Location:** `node/`

### Structure

| Module | Purpose | Test Coverage |
|--------|---------|---------------|
| `src/service.rs` | Service builder, consensus setup | ❌ No tests |
| `src/cli.rs` | Command-line interface | ❌ No tests |
| `src/command.rs` | Subcommand implementations | ❌ No tests |
| `src/cfg/` | Configuration management | ✅ Tests (3) |
| `src/chain_spec/` | Chain specification generation | ❌ No tests |
| `src/inherent_data.rs` | Inherent data providers | ❌ No tests |
| `src/rpc.rs` | RPC endpoint configuration | ❌ No tests |

### Strengths

1. **Comprehensive Service Builder**
   - Proper integration of AURA, GRANDPA, BEEFY consensus
   - MMR gadget integration for bridges
   - Clean separation of full vs light node paths

2. **Configuration Management**
   - TOML-based configuration files
   - Environment variable support
   - Validation utilities with tests

3. **Modular RPC Setup**
   - Substrate standard RPC methods
   - Midnight-specific extensions
   - BEEFY and GRANDPA RPC support

### Concerns

1. **Limited Test Coverage** - Only `cfg/` module has tests
2. **FIXME Comment:**
   ```rust
   // FIXME #1578 make this available through chainspec
   ```

3. **TODO Comments:**
   ```rust
   // TODO: Add metrics
   // TODO: BEEFY(follow up pr)
   ```

### Recommendations

- Add integration tests for service initialization
- Add tests for chain spec generation
- Address FIXME/TODO comments

---

## 13. Runtime Review

**Location:** `runtime/`

### Structure

| Module | Purpose | Test Coverage |
|--------|---------|---------------|
| `src/lib.rs` | Runtime composition, API implementations | ✅ Tests (5+) |
| `src/authorship.rs` | Block authorship configuration | ❌ No tests |
| `src/check_call_filter.rs` | Call filtering logic | ❌ No tests |
| `src/currency.rs` | Currency handling | ❌ No tests |
| `src/migrations.rs` | Runtime migrations | ❌ No tests |
| `src/session_manager.rs` | Session management | ❌ No tests |

### Strengths

1. **Comprehensive Runtime APIs**
   - 20+ runtime API implementations
   - Proper WASM binary generation
   - Hard-fork version support

2. **Benchmark Integration**
   - `define_benchmarks!` macro for 10 pallets
   - Proper weight calculation infrastructure

3. **Governance Configuration**
   - Council and Technical Committee setup
   - Federated authority integration
   - Motion-based proposal system

### Concerns

1. **Large File Size** - `lib.rs` is 1700+ lines
2. **Comment Indicates Incomplete:**
   ```rust
   // TODO: Benchmark all pallets
   ```

### Recommendations

- Consider splitting `lib.rs` into sub-modules
- Complete benchmarking coverage

---

## 14. Tests Directory Review

**Location:** `tests/`

### Structure

| Module | Purpose | Test Coverage |
|--------|---------|---------------|
| `e2e/` | End-to-end integration tests | ✅ 6 scenarios |
| `redemption-skeleton/` | Glacier Drop redemption fixtures | N/A (Aiken) |

### Strengths

1. **Comprehensive E2E Tests**
   - Full registration/deregistration flows
   - Governance contract deployment
   - Cross-chain observation validation
   - Authorization boundary tests

2. **Real Infrastructure Testing**
   - Uses actual Cardano (Ogmios) clients
   - Tests against running Midnight node
   - Validates cross-chain event propagation

### Test Scenarios

| Test | Coverage |
|------|----------|
| `register_for_dust_production` | cNIGHT registration flow |
| `deploy_governance_contracts_and_validate_membership_reset` | Governance |
| `register_2_cardano_same_dust_address_production` | Multi-registration |
| `cnight_produces_dust` | Token minting |
| `deregister_from_dust_production` | Deregistration |
| `alice_cannot_deregister_bob` | Authorization |

---

## 15. Utilities Review

**Location:** `util/`

### Structure

| Module | Purpose | Test Coverage |
|--------|---------|---------------|
| `toolkit/` | Rust CLI for wallet/tx/contract ops | ✅ Tests (5+) |
| `toolkit-js/` | JavaScript CLI for Compact contracts | ✅ Tests (JS) |
| `upgrader/` | HTTP runtime upgrade service | ❌ No tests |
| `documented/` | Doc extraction proc macro | ✅ Tests (1) |

### Strengths

1. **Feature-Complete Toolkit**
   - Wallet management (show, generate)
   - Transaction generation (single, batched)
   - Contract interaction (deploy, call)
   - Genesis generation

2. **Good Test Coverage for Toolkit**
   - CLI argument parsing tests
   - Address generation tests
   - Token type display tests

3. **JavaScript Tooling**
   - Compact contract compilation
   - Integration with TypeScript ecosystem
   - Deploy tests

### Concerns

1. **Upgrader Has No Tests** - Critical service for runtime upgrades
2. **Typo in Code:**
   ```rust
   pub type SignagtureType = ();  // Should be SignatureType
   ```

### Recommendations

- Add tests for upgrader service
- Fix typo in `toolkit/src/lib.rs`

---

## 16. Test Coverage Summary (All Modules)

### Overall Test Distribution

| Module | Test Files | Test Functions | Rating |
|--------|------------|----------------|--------|
| **pallets/** | 5 | ~100 | ✅ Good |
| **runtime/** | 1 | ~5 | ⚠️ Fair |
| **primitives/** | 0 | 0 | ❌ None |
| **ledger/** | 5 | ~10 | ⚠️ Fair |
| **ledger/helpers/** | 2 | ~5 | ⚠️ Fair |
| **node/** | 1 | ~3 | ⚠️ Fair |
| **util/toolkit/** | 4 | ~10 | ✅ Good |
| **util/documented/** | 1 | ~1 | ⚠️ Minimal |
| **tests/e2e/** | 1 | 6 | ✅ Good |
| **TOTAL** | **20** | **~140** | **⚠️ Fair** |

### Module Coverage Matrix

| Module | Unit | Integration | E2E | Benchmarks |
|--------|------|-------------|-----|------------|
| pallets/midnight | ✅ | - | ✅ | ✅ |
| pallets/federated-authority | ✅ | - | ✅ | ✅ |
| pallets/federated-authority-observation | ✅ | - | ✅ | ✅ |
| pallets/cnight-observation | ✅ | - | ✅ | ❌ |
| pallets/midnight-system | ❌ | - | - | ❌ |
| pallets/version | ✅ | - | - | ⚠️ |
| primitives/* | ❌ | - | - | - |
| ledger/ | ⚠️ | - | - | - |
| node/ | ⚠️ | - | - | - |
| runtime/ | ✅ | - | - | ✅ |
| util/toolkit | ✅ | - | - | - |
| util/upgrader | ❌ | - | - | - |

### Priority Test Gaps

1. 🔴 **HIGH:** `pallet-midnight-system` - Zero coverage
2. 🔴 **HIGH:** `util/upgrader` - Zero coverage for critical service
3. 🟠 **MEDIUM:** `primitives/*` - No type validation tests
4. 🟠 **MEDIUM:** `node/service.rs` - No service initialization tests
5. 🟡 **LOW:** `ledger/host_api` - No host function tests

---

## Appendix: File References

| File | Key Findings |
|------|--------------|
| `Cargo.toml` | Workspace config, permissive Clippy lints |
| `deny.toml` | Good security auditing setup |
| `pallets/midnight/src/lib.rs` | Core pallet, needs error handling review |
| `pallets/midnight-system/src/lib.rs` | Missing tests |
| `pallets/federated-authority/` | Excellent test coverage |
| `primitives/midnight/src/lib.rs` | Core traits, no tests |
| `primitives/mainchain-follower/src/lib.rs` | TODO for error types |
| `ledger/src/lib.rs` | Module parameterization, hard-fork support |
| `ledger/src/host_api/mod.rs` | Host functions, no tests |
| `node/src/service.rs` | Service builder, no tests |
| `node/src/cfg/mod.rs` | Configuration, has tests |
| `runtime/src/lib.rs` | Good benchmark integration |
| `tests/e2e/` | Strong E2E test coverage |
| `util/toolkit/src/lib.rs` | Good toolkit, has typo |
| `util/upgrader/` | No tests for critical service |

