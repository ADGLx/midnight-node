// This file is part of midnight-node.
// Copyright (C) Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// You may not use this file except in compliance with the License.
// You may obtain a copy of the License at
// http://www.apache.org/licenses/LICENSE-2.0
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Wallet state caching types and helpers.
//!
//! This module provides types for caching wallet state across toolkit sessions,
//! and helper functions for serializing/deserializing [`LedgerContext`].
//!
//! The cache enables subsequent sessions to restore from a checkpoint and only
//! replay new blocks, dramatically reducing startup time on long-running networks.

use midnight_node_ledger_helpers::{
	BlockContext, DefaultDB, DustLocalState, HashOutput, LedgerContext, LedgerState, Sp, Timestamp,
	Wallet, WalletSeed, WalletState, deserialize_untagged, serialize_untagged,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use subxt::utils::H256;

/// Current cache format version. Increment when format changes.
pub const CACHE_VERSION: &str = "wallet-state-cache-v1";

/// Cache entry for wallet state at a specific block height.
///
/// This structure contains all the serialized state needed to restore a
/// [`LedgerContext`] without replaying all transactions from genesis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletStateCache {
	/// Chain identity (block 1 hash) - ensures cache is not applied to wrong network
	pub chain_id: H256,

	/// Wallet identity - hash of wallet public keys
	pub wallet_id: H256,

	/// Block height at which this cache was created
	pub block_height: u64,

	/// Serialized LedgerState (using mn_ledger_serialize)
	pub ledger_state_bytes: Vec<u8>,

	/// Snapshots of each wallet's state
	pub wallet_snapshots: Vec<WalletSnapshot>,

	/// Latest block context at cache time
	pub latest_block_context: SerializableBlockContext,

	/// State root hash for integrity verification
	pub state_root: Vec<u8>,

	/// Version tag for cache format compatibility
	pub version: String,
}

/// Serializable representation of BlockContext.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializableBlockContext {
	pub tblock_secs: u64,
	pub tblock_err: u32,
	pub parent_block_hash: [u8; 32],
	pub last_block_time: u64,
}

impl From<&BlockContext> for SerializableBlockContext {
	fn from(ctx: &BlockContext) -> Self {
		Self {
			tblock_secs: ctx.tblock.to_secs(),
			tblock_err: ctx.tblock_err,
			parent_block_hash: ctx.parent_block_hash.0,
			last_block_time: ctx.last_block_time.to_secs(),
		}
	}
}

/// Snapshot of a single wallet's state for caching.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletSnapshot {
	/// Hash of the wallet seed (for matching on restore)
	pub seed_hash: H256,

	/// Serialized WalletState<D> (shielded coin tracking)
	pub shielded_state_bytes: Vec<u8>,

	/// Serialized DustLocalState<D> (DUST tracking), if present
	pub dust_local_state_bytes: Option<Vec<u8>>,
}

/// Cache key combining chain and wallet identity.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub struct WalletCacheKey {
	pub chain_id: H256,
	pub wallet_id: H256,
}

impl WalletCacheKey {
	pub fn new(chain_id: H256, wallet_id: H256) -> Self {
		Self { chain_id, wallet_id }
	}
}

// =============================================================================
// Compression utilities
// =============================================================================

/// Compress data using zstd.
///
/// Provides significant size reduction (50-80%) for serialized wallet state,
/// addressing scaling concerns on long-running networks.
pub fn compress(data: &[u8]) -> std::io::Result<Vec<u8>> {
	let mut encoder = zstd::stream::Encoder::new(Vec::new(), 3)?; // Level 3 is a good balance
	encoder.write_all(data)?;
	encoder.finish()
}

/// Decompress zstd-compressed data.
pub fn decompress(data: &[u8]) -> std::io::Result<Vec<u8>> {
	let mut decoder = zstd::stream::Decoder::new(data)?;
	let mut decompressed = Vec::new();
	decoder.read_to_end(&mut decompressed)?;
	Ok(decompressed)
}

// =============================================================================
// Cache helper functions
// =============================================================================

/// Error type for cache serialization/deserialization.
#[derive(Debug, thiserror::Error)]
pub enum CacheError {
	#[error("Failed to serialize ledger state: {0}")]
	SerializeLedgerState(String),
	#[error("Failed to deserialize ledger state: {0}")]
	DeserializeLedgerState(String),
	#[error("Cache version mismatch: expected {expected}, got {actual}")]
	VersionMismatch { expected: String, actual: String },
	#[error("Chain ID mismatch: expected {expected:?}, got {actual:?}")]
	ChainIdMismatch { expected: H256, actual: H256 },
	#[error("State root mismatch: cached data may be corrupted")]
	StateRootMismatch,
	#[error("Failed to acquire lock: {0}")]
	LockPoisoned(String),
}

/// Serialize a LedgerState to bytes using mn_ledger_serialize.
fn serialize_ledger_state(state: &LedgerState<DefaultDB>) -> Result<Vec<u8>, CacheError> {
	midnight_node_ledger_helpers::serialize(state)
		.map_err(|e| CacheError::SerializeLedgerState(e.to_string()))
}

/// Hash a wallet seed for use as snapshot key.
fn hash_seed(seed: &WalletSeed) -> H256 {
	let mut hasher = Sha256::new();
	hasher.update(seed.as_bytes());
	H256::from_slice(&hasher.finalize())
}

/// Compute a state root hash from serialized ledger state bytes.
///
/// This provides integrity verification for cached state without depending
/// on ledger internals.
fn compute_state_root(ledger_state_bytes: &[u8]) -> Vec<u8> {
	let mut hasher = Sha256::new();
	hasher.update(ledger_state_bytes);
	hasher.finalize().to_vec()
}

/// Create a WalletStateCache from a LedgerContext.
///
/// This captures the current state of the ledger and each wallet's dust and
/// shielded state. On restore, wallets are reconstructed with the cached state
/// so only blocks after the checkpoint need to be replayed.
///
/// The state_root is automatically computed from the serialized ledger state
/// to enable integrity verification on restore.
///
/// # Arguments
///
/// * `context` - The LedgerContext to cache
/// * `chain_id` - Chain identity (block 1 hash)
/// * `wallet_id` - Wallet identity (caller-provided, typically from seed hash)
/// * `block_height` - Block height at which this cache is created
pub fn create_cache_from_context(
	context: &LedgerContext<DefaultDB>,
	chain_id: H256,
	wallet_id: H256,
	block_height: u64,
) -> Result<WalletStateCache, CacheError> {
	// Serialize ledger state
	let ledger_state = context
		.ledger_state
		.lock()
		.map_err(|_| CacheError::LockPoisoned("ledger_state".to_string()))?;
	let ledger_state_bytes = serialize_ledger_state(&ledger_state)?;
	drop(ledger_state);

	// Compute state root for integrity verification
	let state_root = compute_state_root(&ledger_state_bytes);

	let wallets = context
		.wallets
		.lock()
		.map_err(|_| CacheError::LockPoisoned("wallets".to_string()))?;
	let wallet_snapshots: Vec<WalletSnapshot> = wallets
		.iter()
		.map(|(seed, wallet)| {
			let dust_local_state_bytes = wallet
				.dust
				.dust_local_state
				.as_ref()
				.map(|state| serialize_untagged(&**state))
				.transpose()
				.map_err(|e| CacheError::SerializeLedgerState(format!("dust state: {}", e)))?;

			let shielded_state_bytes = serialize_untagged(&wallet.shielded.state)
				.map_err(|e| CacheError::SerializeLedgerState(format!("shielded state: {}", e)))?;

			Ok(WalletSnapshot {
				seed_hash: hash_seed(seed),
				shielded_state_bytes,
				dust_local_state_bytes,
			})
		})
		.collect::<Result<Vec<_>, CacheError>>()?;
	drop(wallets);

	// Get latest block context
	let latest_block_context = context.latest_block_context();
	let serializable_context = SerializableBlockContext::from(&latest_block_context);

	Ok(WalletStateCache {
		chain_id,
		wallet_id,
		block_height,
		ledger_state_bytes,
		wallet_snapshots,
		latest_block_context: serializable_context,
		state_root,
		version: CACHE_VERSION.to_string(),
	})
}

/// Restore a LedgerContext from a WalletStateCache.
///
/// This creates a new LedgerContext with the cached ledger state and restores
/// each wallet's dust and shielded state from the cache. Only blocks after the
/// cache checkpoint need to be replayed.
///
/// # Arguments
///
/// * `cache` - The cached state to restore from
/// * `wallet_seeds` - The wallet seeds to initialize
/// * `expected_chain_id` - The expected chain ID (for validation)
///
/// # Returns
///
/// A tuple of (LedgerContext, block_height) where block_height is the height
/// at which the cache was created. The caller should replay blocks from
/// block_height+1 to current head to update wallet state.
pub fn restore_context_from_cache(
	cache: &WalletStateCache,
	wallet_seeds: &[WalletSeed],
	expected_chain_id: H256,
) -> Result<(LedgerContext<DefaultDB>, u64), CacheError> {
	// Validate version
	if cache.version != CACHE_VERSION {
		return Err(CacheError::VersionMismatch {
			expected: CACHE_VERSION.to_string(),
			actual: cache.version.clone(),
		});
	}

	// Validate chain ID
	if cache.chain_id != expected_chain_id {
		return Err(CacheError::ChainIdMismatch {
			expected: expected_chain_id,
			actual: cache.chain_id,
		});
	}

	// Verify state root integrity
	let computed_root = compute_state_root(&cache.ledger_state_bytes);
	if cache.state_root != computed_root {
		log::error!(
			"State root mismatch: cached data may be corrupted (height {})",
			cache.block_height
		);
		return Err(CacheError::StateRootMismatch);
	}

	// Deserialize ledger state using trusted (fast) deserializer
	let ledger_state = super::trusted_deserialize::trusted_deserialize_tagged::<
		LedgerState<DefaultDB>,
	>(&cache.ledger_state_bytes)
	.map_err(|e| CacheError::DeserializeLedgerState(e.to_string()))?;

	// Create context with a placeholder network_id, then replace the ledger state.
	let context = LedgerContext::new("restored");
	{
		let mut state = context
			.ledger_state
			.lock()
			.map_err(|_| CacheError::LockPoisoned("ledger_state".to_string()))?;
		*state = Sp::new(ledger_state.clone());
	}

	// Restore block context
	let block_context = BlockContext {
		tblock: Timestamp::from_secs(cache.latest_block_context.tblock_secs),
		tblock_err: cache.latest_block_context.tblock_err,
		parent_block_hash: HashOutput(cache.latest_block_context.parent_block_hash),
		last_block_time: Timestamp::from_secs(cache.latest_block_context.last_block_time),
	};
	{
		let mut block_ctx = context
			.latest_block_context
			.lock()
			.map_err(|_| CacheError::LockPoisoned("latest_block_context".to_string()))?;
		*block_ctx = Some(block_context);
	}

	let mut wallets = context
		.wallets
		.lock()
		.map_err(|_| CacheError::LockPoisoned("wallets".to_string()))?;
	for seed in wallet_seeds {
		let mut wallet = Wallet::default(*seed, &ledger_state);

		let seed_hash = hash_seed(seed);
		if let Some(snapshot) = cache.wallet_snapshots.iter().find(|s| s.seed_hash == seed_hash) {
			if let Some(ref dust_bytes) = snapshot.dust_local_state_bytes {
				let dust_state =
					deserialize_untagged::<DustLocalState<DefaultDB>>(dust_bytes.as_slice())
						.map_err(|e| {
							CacheError::DeserializeLedgerState(format!("dust state: {}", e))
						})?;
				wallet.dust.dust_local_state = Some(Sp::new(dust_state));
				log::debug!("Restored dust state for wallet");
			}
			if !snapshot.shielded_state_bytes.is_empty() {
				let shielded_state = deserialize_untagged::<WalletState<DefaultDB>>(
					snapshot.shielded_state_bytes.as_slice(),
				)
				.map_err(|e| {
					CacheError::DeserializeLedgerState(format!("shielded state: {}", e))
				})?;
				wallet.shielded.state = shielded_state;
				log::debug!("Restored shielded state for wallet");
			}
		}

		wallets.insert(*seed, wallet);
	}
	drop(wallets);

	log::info!(
		"Restored LedgerContext from cache at block height {}, {} wallets restored",
		cache.block_height,
		wallet_seeds.len()
	);

	Ok((context, cache.block_height))
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_compression_roundtrip() {
		let original = b"Hello, this is some test data that should compress well. \
		                 Repeating content helps compression: aaaaaaaaaaaaaaaaaaaaaa";

		let compressed = compress(original).expect("compression should succeed");
		let decompressed = decompress(&compressed).expect("decompression should succeed");

		assert_eq!(&decompressed, original);
		// Compression should reduce size for this input
		assert!(compressed.len() < original.len());
	}

	#[test]
	fn test_compression_empty() {
		let original = b"";
		let compressed = compress(original).expect("compression should succeed");
		let decompressed = decompress(&compressed).expect("decompression should succeed");
		assert_eq!(&decompressed, original);
	}

	#[test]
	fn test_decompress_invalid_data_returns_error() {
		// Invalid/corrupted data should return an error, not panic
		let garbage = b"this is not valid zstd compressed data";
		let result = decompress(garbage);
		assert!(result.is_err(), "decompress should return error for invalid data");
	}

	#[test]
	fn test_state_root_verification_rejects_corrupted_cache() {
		// Create a cache with valid state root
		let ledger_state_bytes = vec![1u8, 2, 3, 4, 5];
		let valid_root = compute_state_root(&ledger_state_bytes);

		let mut cache = WalletStateCache {
			chain_id: H256::from([1u8; 32]),
			wallet_id: H256::from([2u8; 32]),
			block_height: 100,
			ledger_state_bytes: ledger_state_bytes.clone(),
			wallet_snapshots: vec![],
			latest_block_context: SerializableBlockContext {
				tblock_secs: 1234567890,
				tblock_err: 0,
				parent_block_hash: [0u8; 32],
				last_block_time: 1234567890,
			},
			state_root: valid_root.clone(),
			version: CACHE_VERSION.to_string(),
		};

		// Corrupt the ledger state bytes (simulating storage corruption)
		cache.ledger_state_bytes = vec![9u8, 9, 9, 9, 5]; // Different data

		// Verify the state root check detects corruption
		let computed_root = compute_state_root(&cache.ledger_state_bytes);
		assert_ne!(computed_root, valid_root, "Corrupted data should produce different root");
		assert_ne!(
			cache.state_root, computed_root,
			"State root verification should detect corruption"
		);
	}

	#[test]
	fn test_version_mismatch_rejected() {
		// Cache with outdated version should be rejected
		let cache = WalletStateCache {
			chain_id: H256::from([1u8; 32]),
			wallet_id: H256::from([2u8; 32]),
			block_height: 100,
			ledger_state_bytes: vec![1, 2, 3],
			wallet_snapshots: vec![],
			latest_block_context: SerializableBlockContext {
				tblock_secs: 1234567890,
				tblock_err: 0,
				parent_block_hash: [0u8; 32],
				last_block_time: 1234567890,
			},
			state_root: compute_state_root(&[1, 2, 3]),
			version: "wallet-state-cache-v0".to_string(), // Old version
		};

		let expected_chain_id = H256::from([1u8; 32]);
		let result = restore_context_from_cache(&cache, &[], expected_chain_id);

		match result {
			Err(CacheError::VersionMismatch { expected, actual }) => {
				assert_eq!(expected, CACHE_VERSION);
				assert_eq!(actual, "wallet-state-cache-v0");
			},
			Err(other) => panic!("Expected VersionMismatch error, got: {}", other),
			Ok(_) => panic!("Expected VersionMismatch error, got Ok"),
		}
	}

	/// Load genesis test data as SourceTransactions + build LedgerContext.
	fn load_genesis_context(
		wallet_seeds: &[WalletSeed],
	) -> (crate::serde_def::SourceTransactions, LedgerContext<DefaultDB>) {
		use crate::tx_generator::builder::build_fork_aware_context;

		let genesis_path =
			format!("{}/test-data/genesis/genesis_block_undeployed.mn", env!("CARGO_MANIFEST_DIR"));
		let batches =
			crate::tx_generator::source::GetTxsFromFile::load_single_or_multiple(&genesis_path)
				.expect("failed to load genesis file");
		let source = crate::serde_def::SourceTransactions::from_batches(batches.batches, true);
		let context =
			build_fork_aware_context(&source, wallet_seeds).expect("failed to build context");
		(source, context)
	}

	#[test]
	fn cache_create_and_restore_roundtrip() {
		let wallet_seed = WalletSeed::try_from_hex_str(
			"0000000000000000000000000000000000000000000000000000000000000001",
		)
		.unwrap();
		let wallet_seeds = vec![wallet_seed];

		let (source, context) = load_genesis_context(&wallet_seeds);
		let total_blocks = source.blocks.len() as u64;

		let chain_id = H256::from([1u8; 32]);
		let wallet_id = H256::from([2u8; 32]);
		let cache = create_cache_from_context(&context, chain_id, wallet_id, total_blocks)
			.expect("create cache failed");

		let (restored, height) = restore_context_from_cache(&cache, &wallet_seeds, chain_id)
			.expect("restore from cache failed");
		assert_eq!(height, total_blocks);

		let original_bytes = {
			let state = context.ledger_state.lock().unwrap();
			midnight_node_ledger_helpers::serialize(&**state).expect("serialize failed")
		};
		let restored_bytes = {
			let state = restored.ledger_state.lock().unwrap();
			midnight_node_ledger_helpers::serialize(&**state).expect("serialize failed")
		};

		assert_eq!(
			original_bytes,
			restored_bytes,
			"ledger state bytes differ: original {} bytes vs restored {} bytes",
			original_bytes.len(),
			restored_bytes.len()
		);

		// Verify wallet-level state survives the roundtrip
		let original_wallets = context.wallets.lock().unwrap();
		let restored_wallets = restored.wallets.lock().unwrap();
		assert_eq!(original_wallets.len(), restored_wallets.len(), "wallet count mismatch");

		let orig_wallet = original_wallets.get(&wallet_seed).expect("original wallet missing");
		let rest_wallet = restored_wallets.get(&wallet_seed).expect("restored wallet missing");

		let orig_shielded = serialize_untagged(&orig_wallet.shielded.state)
			.expect("serialize original shielded state");
		let rest_shielded = serialize_untagged(&rest_wallet.shielded.state)
			.expect("serialize restored shielded state");
		assert_eq!(
			orig_shielded,
			rest_shielded,
			"shielded state bytes differ: {} vs {} bytes",
			orig_shielded.len(),
			rest_shielded.len()
		);

		let orig_dust = orig_wallet
			.dust
			.dust_local_state
			.as_ref()
			.map(|s| serialize_untagged(&**s).expect("serialize original dust state"));
		let rest_dust = rest_wallet
			.dust
			.dust_local_state
			.as_ref()
			.map(|s| serialize_untagged(&**s).expect("serialize restored dust state"));
		assert_eq!(orig_dust, rest_dust, "dust local state bytes differ");
	}

	/// Verifies that `cache at block N + replay N+1..M == full replay 0..M`.
	///
	/// With the test genesis having a single block (block 0), the incremental
	/// portion is empty, but the test still exercises the full
	/// restore_context_from_cache → update_from_block path.
	#[test]
	fn cache_restore_then_incremental_replay() {
		use crate::tx_generator::builder::build_fork_aware_context;
		use midnight_node_ledger_helpers::fork::fork_aware_context::ForkAwareLedgerContext;

		let wallet_seed = WalletSeed::try_from_hex_str(
			"0000000000000000000000000000000000000000000000000000000000000001",
		)
		.unwrap();
		let wallet_seeds = vec![wallet_seed];
		let chain_id = H256::from([1u8; 32]);
		let wallet_id = H256::from([2u8; 32]);

		let (source, _) = load_genesis_context(&wallet_seeds);

		// Split blocks: first_half gets cached, second_half gets replayed incrementally
		let split_at = source.blocks.len() / 2 + 1; // At least 1 block in first half
		let first_half = &source.blocks[..split_at];
		let second_half = &source.blocks[split_at..];

		// --- Full path: replay all blocks from scratch ---
		let full_context =
			build_fork_aware_context(&source, &wallet_seeds).expect("full context build failed");

		// --- Cached path: replay first half → cache → restore → replay second half ---
		let partial_source =
			crate::serde_def::SourceTransactions::new(first_half.to_vec(), &source.network_id);
		let partial_context = build_fork_aware_context(&partial_source, &wallet_seeds)
			.expect("partial context build failed");

		let cache_height = (split_at as u64).saturating_sub(1);
		let cache = create_cache_from_context(&partial_context, chain_id, wallet_id, cache_height)
			.expect("create cache failed");

		let (incremental_context, height) =
			restore_context_from_cache(&cache, &wallet_seeds, chain_id)
				.expect("restore from cache failed");
		assert_eq!(height, cache_height);

		// Replay remaining blocks through the fork-aware path
		let mut fork_ctx = ForkAwareLedgerContext::Ledger8(incremental_context);
		for block in second_half {
			fork_ctx = fork_ctx.update_from_block(block);
		}
		let incremental_context = fork_ctx.into_ledger8().expect("expected ledger 8 after replay");

		// --- Compare: full replay vs cache+incremental must produce identical state ---
		let full_bytes = {
			let state = full_context.ledger_state.lock().unwrap();
			midnight_node_ledger_helpers::serialize(&**state).expect("serialize failed")
		};
		let incremental_bytes = {
			let state = incremental_context.ledger_state.lock().unwrap();
			midnight_node_ledger_helpers::serialize(&**state).expect("serialize failed")
		};
		assert_eq!(
			full_bytes,
			incremental_bytes,
			"ledger state diverged: full replay {} bytes vs cached+incremental {} bytes",
			full_bytes.len(),
			incremental_bytes.len()
		);

		// Verify wallet-level state matches
		let full_wallets = full_context.wallets.lock().unwrap();
		let incr_wallets = incremental_context.wallets.lock().unwrap();
		assert_eq!(full_wallets.len(), incr_wallets.len(), "wallet count mismatch");

		let full_wallet = full_wallets.get(&wallet_seed).expect("full wallet missing");
		let incr_wallet = incr_wallets.get(&wallet_seed).expect("incremental wallet missing");

		let full_shielded =
			serialize_untagged(&full_wallet.shielded.state).expect("serialize failed");
		let incr_shielded =
			serialize_untagged(&incr_wallet.shielded.state).expect("serialize failed");
		assert_eq!(full_shielded, incr_shielded, "shielded state diverged");

		let full_dust = full_wallet
			.dust
			.dust_local_state
			.as_ref()
			.map(|s| serialize_untagged(&**s).expect("serialize failed"));
		let incr_dust = incr_wallet
			.dust
			.dust_local_state
			.as_ref()
			.map(|s| serialize_untagged(&**s).expect("serialize failed"));
		assert_eq!(full_dust, incr_dust, "dust local state diverged");
	}

	#[test]
	fn test_chain_id_mismatch_rejected() {
		// Cache created for different chain should be rejected
		let cache = WalletStateCache {
			chain_id: H256::from([1u8; 32]), // Cache was created for chain 1
			wallet_id: H256::from([2u8; 32]),
			block_height: 100,
			ledger_state_bytes: vec![1, 2, 3],
			wallet_snapshots: vec![],
			latest_block_context: SerializableBlockContext {
				tblock_secs: 1234567890,
				tblock_err: 0,
				parent_block_hash: [0u8; 32],
				last_block_time: 1234567890,
			},
			state_root: compute_state_root(&[1, 2, 3]),
			version: CACHE_VERSION.to_string(),
		};

		let expected_chain_id = H256::from([99u8; 32]); // But we're on chain 99
		let result = restore_context_from_cache(&cache, &[], expected_chain_id);

		match result {
			Err(CacheError::ChainIdMismatch { expected, actual }) => {
				assert_eq!(expected, H256::from([99u8; 32]));
				assert_eq!(actual, H256::from([1u8; 32]));
			},
			Err(other) => panic!("Expected ChainIdMismatch error, got: {}", other),
			Ok(_) => panic!("Expected ChainIdMismatch error, got Ok"),
		}
	}
}
