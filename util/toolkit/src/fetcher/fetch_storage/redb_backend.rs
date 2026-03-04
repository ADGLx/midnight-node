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

use std::{any::type_name, cmp::Ordering, path::Path, sync::Arc};

use async_trait::async_trait;
use core::fmt::Debug;
use midnight_node_ledger_helpers::fork::raw_block_data::RawBlockData;
use redb::{Database, Key, ReadableDatabase, ReadableTable, TableDefinition, TypeName, Value};
use serde::{Deserialize, Serialize};
use subxt::utils::H256;
use tokio::sync::RwLock;

use super::FetchStorage;
use crate::fetcher::wallet_state_cache::{
	CachedWalletState, IndividualWalletKey, LedgerSnapshot, LedgerSnapshotKey,
};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct BlockKey {
	chain_id: H256,
	block_number: u64,
}

/// Persistent [`FetchStorage`] backend using [redb](https://github.com/cberner/redb).
///
/// Block data keys/values use BSON via `Serde<T>` wrapper. Wallet state uses
/// manual byte encoding (see [`LedgerSnapshot::to_value_bytes`] / [`CachedWalletState::to_value_bytes`]).
#[derive(Clone)]
pub struct RedbBackend {
	pub db: Arc<RwLock<Database>>,
	pub block_data_table: TableDefinition<'static, Serde<BlockKey>, Serde<RawBlockData>>,
	pub highest_verified_table: TableDefinition<'static, [u8; 32], u64>,
	pub ledger_snapshot_table: TableDefinition<'static, Serde<LedgerSnapshotKey>, &'static [u8]>,
	pub wallet_cache_v2_table: TableDefinition<'static, Serde<IndividualWalletKey>, &'static [u8]>,
}

impl RedbBackend {
	/// Creates or opens a database at the given path. Will fail if open in another process.
	pub fn new(path: impl AsRef<Path>) -> Self {
		let p = path.as_ref();
		if let Some(parent) = p.parent() {
			std::fs::create_dir_all(parent)
				.expect("failed to create parent dir for redb fetch cache");
		}
		Self {
			db: Arc::new(RwLock::new(
				Database::create(path).expect("failed to create database - is it already open?"),
			)),
			block_data_table: TableDefinition::new("raw_block_data_v1"),
			highest_verified_table: TableDefinition::new("highest_verified"),
			ledger_snapshot_table: TableDefinition::new("ledger_snapshots_v2"),
			wallet_cache_v2_table: TableDefinition::new("wallet_cache_v2"),
		}
	}
}

#[async_trait]
impl FetchStorage for RedbBackend {
	async fn get_block_data(&self, chain_id: H256, block_number: u64) -> Option<RawBlockData> {
		let read_txn = self.db.read().await.begin_read().expect("failed to begin read txn");
		let Ok(table) = read_txn.open_table(self.block_data_table) else { return None };
		table
			.get(BlockKey { chain_id, block_number })
			.expect("failed to get from table")
			.map(|a| a.value())
	}
	async fn get_block_data_range(
		&self,
		chain_id: H256,
		range: impl Iterator<Item = u64> + Send,
	) -> Vec<Option<RawBlockData>> {
		let read_txn = self.db.read().await.begin_read().expect("failed to begin read txn");
		let Ok(table) = read_txn.open_table(self.block_data_table) else {
			return std::iter::repeat_n(None, range.count()).collect();
		};
		range
			.into_iter()
			.map(|block_number| {
				table
					.get(BlockKey { chain_id, block_number })
					.expect("failed to get from table")
					.map(|a| a.value())
			})
			.collect()
	}

	async fn insert_block_data(&self, chain_id: H256, block_number: u64, block: RawBlockData) {
		let write_txn = self.db.write().await.begin_write().expect("failed to begin write txn");
		{
			let mut table =
				write_txn.open_table(self.block_data_table).expect("failed to open table");
			table
				.insert(BlockKey { chain_id, block_number }, block)
				.expect("failed to insert block");
		}
		write_txn.commit().expect("failed to commit write")
	}

	async fn insert_block_data_range(
		&self,
		chain_id: H256,
		range: impl Iterator<Item = (u64, RawBlockData)> + Send,
	) {
		let write_txn = self.db.write().await.begin_write().expect("failed to begin write txn");
		{
			let mut table =
				write_txn.open_table(self.block_data_table).expect("failed to open table");
			for (block_number, block) in range {
				table
					.insert(BlockKey { chain_id, block_number }, block)
					.expect("failed to insert block");
			}
		}
		write_txn.commit().expect("failed to commit write")
	}

	async fn get_highest_verified_block(&self, chain_id: H256) -> Option<u64> {
		let read_txn = self.db.read().await.begin_read().expect("failed to begin read txn");
		let Ok(table) = read_txn.open_table(self.highest_verified_table) else { return None };
		table.get(&chain_id.0).expect("failed to get from table").map(|a| a.value())
	}

	async fn set_highest_verified_block(&self, chain_id: H256, height: u64) {
		let write_txn = self.db.write().await.begin_write().expect("failed to begin write txn");
		{
			let mut table =
				write_txn.open_table(self.highest_verified_table).expect("failed to open table");
			table.insert(&chain_id.0, height).expect("failed to insert highest verified");
		}
		write_txn.commit().expect("failed to commit write")
	}

	// =========================================================================
	// Per-wallet cache (v2) — ledger snapshots
	// =========================================================================

	async fn get_ledger_snapshot(
		&self,
		chain_id: H256,
		block_height: u64,
	) -> Option<LedgerSnapshot> {
		let key = LedgerSnapshotKey { chain_id, block_height };
		let read_txn = match self.db.read().await.begin_read() {
			Ok(txn) => txn,
			Err(e) => {
				log::warn!("Failed to begin read transaction for ledger snapshot: {e}");
				return None;
			},
		};
		let Ok(table) = read_txn.open_table(self.ledger_snapshot_table) else { return None };

		let raw = match table.get(key) {
			Ok(Some(data)) => data,
			Ok(None) => return None,
			Err(e) => {
				log::warn!("Failed to get ledger snapshot: {e}");
				return None;
			},
		};

		match LedgerSnapshot::from_value_bytes(raw.value(), block_height) {
			Ok(snapshot) => Some(snapshot),
			Err(e) => {
				log::warn!("Failed to decode ledger snapshot: {e}");
				None
			},
		}
	}

	async fn set_ledger_snapshot(&self, chain_id: H256, snapshot: LedgerSnapshot) {
		let key = LedgerSnapshotKey { chain_id, block_height: snapshot.block_height };
		let block_height = snapshot.block_height;
		let encoded = match snapshot.to_value_bytes() {
			Ok(b) => b,
			Err(e) => {
				log::warn!("Failed to serialize ledger snapshot: {e}");
				return;
			},
		};

		let write_txn = match self.db.write().await.begin_write() {
			Ok(txn) => txn,
			Err(e) => {
				log::warn!("Failed to begin write transaction for ledger snapshot: {e}");
				return;
			},
		};
		{
			let mut table = match write_txn.open_table(self.ledger_snapshot_table) {
				Ok(t) => t,
				Err(e) => {
					log::warn!("Failed to open ledger snapshot table: {e}");
					return;
				},
			};
			if let Err(e) = table.insert(key, encoded.as_slice()) {
				log::warn!("Failed to insert ledger snapshot: {e}");
				return;
			}
		}
		if let Err(e) = write_txn.commit() {
			log::warn!("Failed to commit ledger snapshot write: {e}");
			return;
		}

		log::info!("Cached ledger snapshot at block {} ({} bytes)", block_height, encoded.len());
	}

	async fn get_latest_ledger_height(&self, chain_id: H256) -> Option<u64> {
		let read_txn = match self.db.read().await.begin_read() {
			Ok(txn) => txn,
			Err(e) => {
				log::warn!("Failed to begin read transaction for latest ledger height: {e}");
				return None;
			},
		};
		let Ok(table) = read_txn.open_table(self.ledger_snapshot_table) else { return None };

		let start = LedgerSnapshotKey { chain_id, block_height: 0 };
		let end = LedgerSnapshotKey { chain_id, block_height: i64::MAX as u64 };
		let mut range = match table.range(start..=end) {
			Ok(range) => range,
			Err(e) => {
				log::warn!("Failed to range-query ledger snapshot table: {e}");
				return None;
			},
		};

		range.next_back().and_then(|entry| {
			let (key, _) = entry.ok()?;
			Some(key.value().block_height)
		})
	}

	// =========================================================================
	// Per-wallet cache (v2) — individual wallets
	// =========================================================================

	async fn get_wallet_states(
		&self,
		chain_id: H256,
		seed_hashes: &[H256],
	) -> Vec<Option<CachedWalletState>> {
		let read_txn = match self.db.read().await.begin_read() {
			Ok(txn) => txn,
			Err(e) => {
				log::warn!("Failed to begin read transaction for wallet states: {e}");
				return seed_hashes.iter().map(|_| None).collect();
			},
		};
		let Ok(table) = read_txn.open_table(self.wallet_cache_v2_table) else {
			return seed_hashes.iter().map(|_| None).collect();
		};

		seed_hashes
			.iter()
			.map(|&seed_hash| {
				let key = IndividualWalletKey { chain_id, seed_hash };
				match table.get(key) {
					Ok(Some(data)) => {
						match CachedWalletState::from_value_bytes(data.value(), seed_hash) {
							Ok(cached) => Some(cached),
							Err(e) => {
								log::warn!("Failed to decode wallet state: {e}");
								None
							},
						}
					},
					Ok(None) => None,
					Err(e) => {
						log::warn!("Failed to get wallet state: {e}");
						None
					},
				}
			})
			.collect()
	}

	async fn set_wallet_states(&self, chain_id: H256, wallets: &[CachedWalletState]) {
		if wallets.is_empty() {
			return;
		}

		let write_txn = match self.db.write().await.begin_write() {
			Ok(txn) => txn,
			Err(e) => {
				log::warn!("Failed to begin write transaction for wallet states: {e}");
				return;
			},
		};
		{
			let mut table = match write_txn.open_table(self.wallet_cache_v2_table) {
				Ok(t) => t,
				Err(e) => {
					log::warn!("Failed to open wallet cache table: {e}");
					return;
				},
			};

			for wallet in wallets {
				let key = IndividualWalletKey { chain_id, seed_hash: wallet.seed_hash };
				let encoded = match wallet.to_value_bytes() {
					Ok(b) => b,
					Err(e) => {
						log::warn!(
							"Failed to serialize wallet state for {:?}: {e}",
							wallet.seed_hash
						);
						continue;
					},
				};
				if let Err(e) = table.insert(key, encoded.as_slice()) {
					log::warn!("Failed to insert wallet state: {e}");
				}
			}
		}
		if let Err(e) = write_txn.commit() {
			log::warn!("Failed to commit wallet states write: {e}");
			return;
		}

		log::info!("Saved {} wallet cache entries", wallets.len());
	}

	async fn delete_wallet_states(&self, chain_id: H256, seed_hashes: &[H256]) {
		if seed_hashes.is_empty() {
			return;
		}

		let write_txn = match self.db.write().await.begin_write() {
			Ok(txn) => txn,
			Err(e) => {
				log::warn!("Failed to begin write transaction for wallet state deletion: {e}");
				return;
			},
		};
		{
			if let Ok(mut table) = write_txn.open_table(self.wallet_cache_v2_table) {
				for &seed_hash in seed_hashes {
					let key = IndividualWalletKey { chain_id, seed_hash };
					let _ = table.remove(key);
				}
			}
		}
		if let Err(e) = write_txn.commit() {
			log::warn!("Failed to commit wallet state deletion: {e}");
		}
	}

	async fn gc_ledger_snapshots(&self, chain_id: H256, keep_heights: &[u64]) {
		let write_txn = match self.db.write().await.begin_write() {
			Ok(txn) => txn,
			Err(e) => {
				log::warn!("Failed to begin write transaction for GC: {e}");
				return;
			},
		};
		{
			let Ok(mut table) = write_txn.open_table(self.ledger_snapshot_table) else {
				return;
			};

			// Collect keys to remove
			let keys_to_remove: Vec<LedgerSnapshotKey> = {
				let start = LedgerSnapshotKey { chain_id, block_height: 0 };
				let end = LedgerSnapshotKey { chain_id, block_height: i64::MAX as u64 };
				let iter = match table.range(start..=end) {
					Ok(iter) => iter,
					Err(e) => {
						log::warn!("Failed to iterate ledger snapshots for GC: {e}");
						return;
					},
				};
				iter.filter_map(|entry| {
					let (key, _) = entry.ok()?;
					let key = key.value();
					if !keep_heights.contains(&key.block_height) { Some(key) } else { None }
				})
				.collect()
			};

			let count = keys_to_remove.len();
			for key in keys_to_remove {
				let _ = table.remove(key);
			}

			if count > 0 {
				log::info!("GC: removed {} stale ledger snapshots", count);
			}
		}
		if let Err(e) = write_txn.commit() {
			log::warn!("Failed to commit GC: {e}");
		}
	}

	async fn get_all_cached_wallet_heights(&self, chain_id: H256) -> Vec<u64> {
		let read_txn = match self.db.read().await.begin_read() {
			Ok(txn) => txn,
			Err(e) => {
				log::warn!("Failed to begin read transaction for cached wallet heights: {e}");
				return Vec::new();
			},
		};
		let Ok(table) = read_txn.open_table(self.wallet_cache_v2_table) else {
			return Vec::new();
		};

		// Range query scoped to this chain_id — avoids full table scan.
		// IndividualWalletKey is Ord-ordered by (chain_id, seed_hash).
		let start = IndividualWalletKey { chain_id, seed_hash: H256::zero() };
		let end = IndividualWalletKey { chain_id, seed_hash: H256::from([0xFF; 32]) };
		let iter = match table.range(start..=end) {
			Ok(iter) => iter,
			Err(e) => {
				log::warn!("Failed to range-query wallet cache for heights: {e}");
				return Vec::new();
			},
		};

		let mut heights = std::collections::HashSet::new();
		for entry in iter {
			let (_, value) = match entry {
				Ok(e) => e,
				Err(_) => continue,
			};
			if let Some(h) = CachedWalletState::block_height_from_value_bytes(value.value()) {
				heights.insert(h);
			}
		}

		heights.into_iter().collect()
	}
}

/// Wrapper type to handle keys and values using bincode serialization
#[derive(Debug)]
pub struct Serde<T>(pub T);

impl<T> Value for Serde<T>
where
	for<'a> T: Debug + Serialize + Deserialize<'a>,
{
	type SelfType<'a>
		= T
	where
		Self: 'a;

	type AsBytes<'a>
		= Vec<u8>
	where
		Self: 'a;

	fn fixed_width() -> Option<usize> {
		None
	}

	fn from_bytes<'a>(data: &'a [u8]) -> Self::SelfType<'a>
	where
		Self: 'a,
	{
		bson::deserialize_from_slice(&data).unwrap()
	}

	fn as_bytes<'a, 'b: 'a>(value: &'a Self::SelfType<'b>) -> Self::AsBytes<'a>
	where
		Self: 'a,
		Self: 'b,
	{
		bson::serialize_to_vec(&value).unwrap()
	}

	fn type_name() -> TypeName {
		TypeName::new(&format!("Serde<{}>", type_name::<T>()))
	}
}

impl<T> Key for Serde<T>
where
	for<'a> T: Debug + Deserialize<'a> + Serialize + Ord,
{
	fn compare(data1: &[u8], data2: &[u8]) -> Ordering {
		Self::from_bytes(data1).cmp(&Self::from_bytes(data2))
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::fetcher::fetch_storage::WalletStateCaching;
	use crate::fetcher::wallet_state_cache::{
		CachedWalletState, LedgerSnapshot, SerializableBlockContext,
	};
	use tempfile::tempdir;

	fn create_test_ledger_snapshot(block_height: u64) -> LedgerSnapshot {
		LedgerSnapshot {
			block_height,
			ledger_state_bytes: vec![0u8; 1000],
			latest_block_context: SerializableBlockContext {
				tblock_secs: 1234567890,
				tblock_err: 0,
				parent_block_hash: [4u8; 32],
				last_block_time: 1234567890,
			},
			state_root: [5u8; 32],
		}
	}

	fn create_test_wallet_state(seed_hash: H256, block_height: u64) -> CachedWalletState {
		CachedWalletState {
			seed_hash,
			block_height,
			shielded_state_bytes: vec![10u8; 200],
			dust_local_state_bytes: Some(vec![20u8; 100]),
		}
	}

	#[tokio::test]
	async fn test_redb_ledger_snapshot_roundtrip() {
		let dir = tempdir().unwrap();
		let db_path = dir.path().join("test.db");
		let backend = RedbBackend::new(&db_path);

		let chain_id = H256::from([1u8; 32]);

		// Initially no snapshot
		assert!(WalletStateCaching::get_ledger_snapshot(&backend, chain_id, 100).await.is_none());

		// Save snapshot
		let snapshot = create_test_ledger_snapshot(100);
		WalletStateCaching::set_ledger_snapshot(&backend, chain_id, snapshot.clone()).await;

		// Retrieve snapshot
		let retrieved = WalletStateCaching::get_ledger_snapshot(&backend, chain_id, 100).await;
		assert!(retrieved.is_some());
		let retrieved = retrieved.unwrap();
		assert_eq!(retrieved.block_height, 100);
		assert_eq!(retrieved.ledger_state_bytes, snapshot.ledger_state_bytes);
		assert_eq!(retrieved.state_root, snapshot.state_root);
	}

	#[tokio::test]
	async fn test_redb_latest_ledger_height() {
		let dir = tempdir().unwrap();
		let db_path = dir.path().join("test.db");
		let backend = RedbBackend::new(&db_path);

		let chain_id = H256::from([1u8; 32]);

		assert!(WalletStateCaching::get_latest_ledger_height(&backend, chain_id).await.is_none());

		WalletStateCaching::set_ledger_snapshot(
			&backend,
			chain_id,
			create_test_ledger_snapshot(100),
		)
		.await;
		WalletStateCaching::set_ledger_snapshot(
			&backend,
			chain_id,
			create_test_ledger_snapshot(200),
		)
		.await;

		let height = WalletStateCaching::get_latest_ledger_height(&backend, chain_id).await;
		assert_eq!(height, Some(200));
	}

	#[tokio::test]
	async fn test_redb_wallet_states_batch() {
		let dir = tempdir().unwrap();
		let db_path = dir.path().join("test.db");
		let backend = RedbBackend::new(&db_path);

		let chain_id = H256::from([1u8; 32]);
		let seed_hash_1 = H256::from([2u8; 32]);
		let seed_hash_2 = H256::from([3u8; 32]);
		let seed_hash_3 = H256::from([4u8; 32]);

		// Initially all None
		let results = WalletStateCaching::get_wallet_states(
			&backend,
			chain_id,
			&[seed_hash_1, seed_hash_2, seed_hash_3],
		)
		.await;
		assert_eq!(results.len(), 3);
		assert!(results.iter().all(|r| r.is_none()));

		// Save two wallets
		let wallet_1 = create_test_wallet_state(seed_hash_1, 100);
		let wallet_2 = create_test_wallet_state(seed_hash_2, 200);
		WalletStateCaching::set_wallet_states(
			&backend,
			chain_id,
			&[wallet_1.clone(), wallet_2.clone()],
		)
		.await;

		// Batch retrieve: 1 and 2 exist, 3 doesn't
		let results = WalletStateCaching::get_wallet_states(
			&backend,
			chain_id,
			&[seed_hash_1, seed_hash_2, seed_hash_3],
		)
		.await;
		assert!(results[0].is_some());
		assert_eq!(results[0].as_ref().unwrap().block_height, 100);
		assert!(results[1].is_some());
		assert_eq!(results[1].as_ref().unwrap().block_height, 200);
		assert!(results[2].is_none());
	}

	#[tokio::test]
	async fn test_redb_delete_wallet_states() {
		let dir = tempdir().unwrap();
		let db_path = dir.path().join("test.db");
		let backend = RedbBackend::new(&db_path);

		let chain_id = H256::from([1u8; 32]);
		let seed_hash_1 = H256::from([2u8; 32]);
		let seed_hash_2 = H256::from([3u8; 32]);

		let wallet_1 = create_test_wallet_state(seed_hash_1, 100);
		let wallet_2 = create_test_wallet_state(seed_hash_2, 200);
		WalletStateCaching::set_wallet_states(&backend, chain_id, &[wallet_1, wallet_2]).await;

		// Delete one
		WalletStateCaching::delete_wallet_states(&backend, chain_id, &[seed_hash_1]).await;

		let results =
			WalletStateCaching::get_wallet_states(&backend, chain_id, &[seed_hash_1, seed_hash_2])
				.await;
		assert!(results[0].is_none());
		assert!(results[1].is_some());
	}

	#[tokio::test]
	async fn test_redb_gc_ledger_snapshots() {
		let dir = tempdir().unwrap();
		let db_path = dir.path().join("test.db");
		let backend = RedbBackend::new(&db_path);

		let chain_id = H256::from([1u8; 32]);

		// Save snapshots at heights 100, 200, 300
		for h in [100, 200, 300] {
			WalletStateCaching::set_ledger_snapshot(
				&backend,
				chain_id,
				create_test_ledger_snapshot(h),
			)
			.await;
		}

		// GC keeping only height 200
		WalletStateCaching::gc_ledger_snapshots(&backend, chain_id, &[200]).await;

		assert!(WalletStateCaching::get_ledger_snapshot(&backend, chain_id, 100).await.is_none());
		assert!(WalletStateCaching::get_ledger_snapshot(&backend, chain_id, 200).await.is_some());
		assert!(WalletStateCaching::get_ledger_snapshot(&backend, chain_id, 300).await.is_none());
	}

	#[tokio::test]
	async fn test_redb_wallet_state_update() {
		let dir = tempdir().unwrap();
		let db_path = dir.path().join("test.db");
		let backend = RedbBackend::new(&db_path);

		let chain_id = H256::from([1u8; 32]);
		let seed_hash = H256::from([2u8; 32]);

		// Save at height 100
		let wallet_1 = create_test_wallet_state(seed_hash, 100);
		WalletStateCaching::set_wallet_states(&backend, chain_id, &[wallet_1]).await;

		let results = WalletStateCaching::get_wallet_states(&backend, chain_id, &[seed_hash]).await;
		assert_eq!(results[0].as_ref().unwrap().block_height, 100);

		// Update to height 200
		let wallet_2 = create_test_wallet_state(seed_hash, 200);
		WalletStateCaching::set_wallet_states(&backend, chain_id, &[wallet_2]).await;

		let results = WalletStateCaching::get_wallet_states(&backend, chain_id, &[seed_hash]).await;
		assert_eq!(results[0].as_ref().unwrap().block_height, 200);
	}

	#[tokio::test]
	async fn test_redb_concurrent_access() {
		let dir = tempdir().unwrap();
		let db_path = dir.path().join("test.db");
		let backend = RedbBackend::new(&db_path);

		let chain_id = H256::from([1u8; 32]);
		let num_wallets = 10u8;
		let num_operations = 5u64;

		let mut handles = vec![];
		for wallet_idx in 0..num_wallets {
			let backend_clone = backend.clone();
			let seed_hash = H256::from([wallet_idx; 32]);

			let handle = tokio::spawn(async move {
				for op in 0..num_operations {
					let wallet = CachedWalletState {
						seed_hash,
						block_height: wallet_idx as u64 * 100 + op,
						shielded_state_bytes: vec![wallet_idx; 100],
						dust_local_state_bytes: None,
					};

					WalletStateCaching::set_wallet_states(&backend_clone, chain_id, &[wallet])
						.await;

					let results = WalletStateCaching::get_wallet_states(
						&backend_clone,
						chain_id,
						&[seed_hash],
					)
					.await;
					assert!(results[0].is_some(), "Wallet {} should have cache", wallet_idx);
				}
			});
			handles.push(handle);
		}

		for handle in handles {
			handle.await.expect("Task should complete successfully");
		}

		// Verify all wallets have their final state
		let all_hashes: Vec<H256> = (0..num_wallets).map(|i| H256::from([i; 32])).collect();
		let results = WalletStateCaching::get_wallet_states(&backend, chain_id, &all_hashes).await;
		assert!(results.iter().all(|r| r.is_some()));
	}
}
