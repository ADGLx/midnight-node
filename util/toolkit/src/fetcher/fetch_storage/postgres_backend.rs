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

use async_trait::async_trait;
use midnight_node_ledger_helpers::fork::raw_block_data::RawBlockData;
use sqlx::{
	PgPool, Row,
	postgres::{PgPoolOptions, PgRow},
};
use subxt::utils::H256;

use super::FetchStorage;
use crate::fetcher::wallet_state_cache::{CachedWalletState, LedgerSnapshot};

/// Persistent [`FetchStorage`] backend using PostgreSQL.
///
/// Block data uses BSON serialization. Wallet cache uses manual byte encoding.
/// Uses sqlx connection pooling.
#[derive(Clone)]
pub struct PostgresBackend {
	pool: PgPool,
}

impl PostgresBackend {
	/// Creates a new backend and initializes tables. Panics on connection failure.
	pub async fn new(database_url: &str) -> Self {
		let pool = PgPoolOptions::new()
			.max_connections(10)
			.connect(database_url)
			.await
			.expect("failed to create database pool");

		let backend = Self { pool };

		backend.init_tables().await;
		backend
	}

	/// Creates a new backend with an existing connection pool.
	pub async fn with_pool(pool: PgPool) -> Self {
		let backend = Self { pool };

		backend.init_tables().await;
		backend
	}

	/// Creates required tables if they don't exist.
	///
	/// Uses a PostgreSQL advisory lock to prevent concurrent `CREATE TABLE IF NOT EXISTS`
	/// from racing on the implicit composite type creation.
	async fn init_tables(&self) {
		let mut tx = self.pool.begin().await.expect("failed to begin transaction");

		// Acquire a session-level advisory lock (released at end of transaction)
		sqlx::query("SELECT pg_advisory_xact_lock(8675309)")
			.execute(&mut *tx)
			.await
			.expect("failed to acquire advisory lock");

		sqlx::query(
			r#"
            CREATE TABLE IF NOT EXISTS raw_block_data_v1 (
                chain_id BYTEA NOT NULL,
                block_number BIGINT NOT NULL,
                data BYTEA NOT NULL,
                PRIMARY KEY (chain_id, block_number)
            )
            "#,
		)
		.execute(&mut *tx)
		.await
		.expect("failed to create raw_block_data_v1 table");

		sqlx::query(
			r#"
            CREATE TABLE IF NOT EXISTS highest_verified (
                chain_id BYTEA PRIMARY KEY,
                height BIGINT NOT NULL
            )
            "#,
		)
		.execute(&mut *tx)
		.await
		.expect("failed to create highest_verified table");

		sqlx::query(
			r#"
            CREATE TABLE IF NOT EXISTS ledger_snapshots_v2 (
                chain_id BYTEA NOT NULL,
                block_height BIGINT NOT NULL,
                data BYTEA NOT NULL,
                updated_at TIMESTAMP DEFAULT NOW(),
                PRIMARY KEY (chain_id, block_height)
            )
            "#,
		)
		.execute(&mut *tx)
		.await
		.expect("failed to create ledger_snapshots_v2 table");

		sqlx::query(
			r#"
            CREATE TABLE IF NOT EXISTS wallet_cache_v2 (
                chain_id BYTEA NOT NULL,
                seed_hash BYTEA NOT NULL,
                block_height BIGINT NOT NULL,
                data BYTEA NOT NULL,
                updated_at TIMESTAMP DEFAULT NOW(),
                PRIMARY KEY (chain_id, seed_hash)
            )
            "#,
		)
		.execute(&mut *tx)
		.await
		.expect("failed to create wallet_cache_v2 table");

		// Drop legacy v1 wallet state cache table and stale indexes
		sqlx::query("DROP TABLE IF EXISTS wallet_state_cache")
			.execute(&mut *tx)
			.await
			.expect("failed to drop legacy wallet_state_cache table");

		sqlx::query("DROP INDEX IF EXISTS idx_raw_block_data_v1_chain_number")
			.execute(&mut *tx)
			.await
			.expect("failed to drop legacy idx_raw_block_data_v1_chain_number index");

		sqlx::query("DROP INDEX IF EXISTS idx_wallet_state_chain")
			.execute(&mut *tx)
			.await
			.expect("failed to drop legacy idx_wallet_state_chain index");

		tx.commit().await.expect("failed to commit init_tables transaction");
	}

	fn serialize_block_data(block: &RawBlockData) -> Vec<u8> {
		bson::serialize_to_vec(block).expect("failed to serialize block data")
	}

	fn deserialize_block_data(data: &[u8]) -> RawBlockData {
		bson::deserialize_from_slice(data).expect("failed to deserialize block data")
	}
}

#[async_trait]
impl FetchStorage for PostgresBackend {
	async fn get_block_data(&self, chain_id: H256, block_number: u64) -> Option<RawBlockData> {
		let result: Option<PgRow> = sqlx::query(
			r#"
            SELECT data FROM raw_block_data_v1
            WHERE chain_id = $1 AND block_number = $2
            "#,
		)
		.bind(chain_id.0.as_slice())
		.bind(block_number as i64)
		.fetch_optional(&self.pool)
		.await
		.expect("failed to query block data");

		result.map(|row| {
			let data: Vec<u8> = row.get("data");
			Self::deserialize_block_data(&data)
		})
	}

	async fn get_block_data_range(
		&self,
		chain_id: H256,
		range: impl Iterator<Item = u64> + Send,
	) -> Vec<Option<RawBlockData>> {
		let block_numbers: Vec<u64> = range.collect();

		if block_numbers.is_empty() {
			return Vec::new();
		}

		let block_numbers_i64: Vec<i64> = block_numbers.iter().map(|&n| n as i64).collect();

		let rows: Vec<PgRow> = sqlx::query(
			r#"
            SELECT bd.data
            FROM UNNEST($2::BIGINT[]) WITH ORDINALITY AS bn(block_number, ord)
            LEFT JOIN raw_block_data_v1 bd ON bd.chain_id = $1 AND bd.block_number = bn.block_number
            ORDER BY bn.ord
            "#,
		)
		.bind(chain_id.0.as_slice())
		.bind(&block_numbers_i64)
		.fetch_all(&self.pool)
		.await
		.expect("failed to query block data range");

		rows.into_iter()
			.map(|row| {
				let data: Option<Vec<u8>> = row.get("data");
				data.map(|d| Self::deserialize_block_data(&d))
			})
			.collect()
	}

	async fn insert_block_data(&self, chain_id: H256, block_number: u64, block: RawBlockData) {
		let data = Self::serialize_block_data(&block);

		sqlx::query(
			r#"
            INSERT INTO raw_block_data_v1 (chain_id, block_number, data)
            VALUES ($1, $2, $3)
            ON CONFLICT (chain_id, block_number)
            DO UPDATE SET data = EXCLUDED.data
            "#,
		)
		.bind(chain_id.0.as_slice())
		.bind(block_number as i64)
		.bind(&data)
		.execute(&self.pool)
		.await
		.expect("failed to insert block data");
	}

	async fn insert_block_data_range(
		&self,
		chain_id: H256,
		range: impl Iterator<Item = (u64, RawBlockData)> + Send,
	) {
		let blocks: Vec<(u64, RawBlockData)> = range.collect();

		if blocks.is_empty() {
			return;
		}

		let mut tx = self.pool.begin().await.expect("failed to begin transaction");

		for (block_number, block) in blocks {
			let data = Self::serialize_block_data(&block);

			sqlx::query(
				r#"
                INSERT INTO raw_block_data_v1 (chain_id, block_number, data)
                VALUES ($1, $2, $3)
                ON CONFLICT (chain_id, block_number)
                DO UPDATE SET data = EXCLUDED.data
                "#,
			)
			.bind(chain_id.0.as_slice())
			.bind(block_number as i64)
			.bind(&data)
			.execute(&mut *tx)
			.await
			.expect("failed to insert block data");
		}

		tx.commit().await.expect("failed to commit transaction");
	}

	async fn get_highest_verified_block(&self, chain_id: H256) -> Option<u64> {
		let result: Option<PgRow> = sqlx::query(
			r#"
            SELECT height FROM highest_verified
            WHERE chain_id = $1
            "#,
		)
		.bind(chain_id.0.as_slice())
		.fetch_optional(&self.pool)
		.await
		.expect("failed to query highest verified block");

		result.map(|row| {
			let height: i64 = row.get("height");
			height as u64
		})
	}

	async fn set_highest_verified_block(&self, chain_id: H256, height: u64) {
		sqlx::query(
			r#"
            INSERT INTO highest_verified (chain_id, height)
            VALUES ($1, $2)
            ON CONFLICT (chain_id)
            DO UPDATE SET height = EXCLUDED.height
            "#,
		)
		.bind(chain_id.0.as_slice())
		.bind(height as i64)
		.execute(&self.pool)
		.await
		.expect("failed to set highest verified block");
	}

	// =========================================================================
	// Per-wallet cache (v2) — ledger snapshots
	// =========================================================================

	async fn get_ledger_snapshot(
		&self,
		chain_id: H256,
		block_height: u64,
	) -> Option<LedgerSnapshot> {
		let result: Option<PgRow> = match sqlx::query(
			r#"
            SELECT data FROM ledger_snapshots_v2
            WHERE chain_id = $1 AND block_height = $2
            "#,
		)
		.bind(chain_id.0.as_slice())
		.bind(block_height as i64)
		.fetch_optional(&self.pool)
		.await
		{
			Ok(row) => row,
			Err(e) => {
				log::warn!("Failed to query ledger snapshot: {e}");
				return None;
			},
		};

		result.and_then(|row| {
			let data: Vec<u8> = row.get("data");
			match LedgerSnapshot::from_value_bytes(&data, block_height) {
				Ok(snapshot) => Some(snapshot),
				Err(e) => {
					log::warn!("Failed to decode ledger snapshot: {e}");
					None
				},
			}
		})
	}

	async fn set_ledger_snapshot(&self, chain_id: H256, snapshot: LedgerSnapshot) {
		let block_height = snapshot.block_height;
		let encoded = match snapshot.to_value_bytes() {
			Ok(b) => b,
			Err(e) => {
				log::warn!("Failed to serialize ledger snapshot: {e}");
				return;
			},
		};

		if let Err(e) = sqlx::query(
			r#"
            INSERT INTO ledger_snapshots_v2 (chain_id, block_height, data, updated_at)
            VALUES ($1, $2, $3, NOW())
            ON CONFLICT (chain_id, block_height)
            DO UPDATE SET data = EXCLUDED.data, updated_at = NOW()
            "#,
		)
		.bind(chain_id.0.as_slice())
		.bind(block_height as i64)
		.bind(&encoded)
		.execute(&self.pool)
		.await
		{
			log::warn!("Failed to set ledger snapshot: {e}");
			return;
		}

		log::info!("Saved ledger snapshot at block {} ({} bytes)", block_height, encoded.len());
	}

	async fn get_latest_ledger_height(&self, chain_id: H256) -> Option<u64> {
		let result: Option<PgRow> = match sqlx::query(
			r#"
            SELECT MAX(block_height) as max_height FROM ledger_snapshots_v2
            WHERE chain_id = $1
            "#,
		)
		.bind(chain_id.0.as_slice())
		.fetch_optional(&self.pool)
		.await
		{
			Ok(row) => row,
			Err(e) => {
				log::warn!("Failed to query latest ledger height: {e}");
				return None;
			},
		};

		result.and_then(|row| {
			let height: Option<i64> = row.get("max_height");
			height.map(|h| h as u64)
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
		if seed_hashes.is_empty() {
			return Vec::new();
		}

		let seed_hash_bytes: Vec<Vec<u8>> = seed_hashes.iter().map(|h| h.0.to_vec()).collect();

		let rows: Vec<PgRow> = match sqlx::query(
			r#"
            SELECT seed_hash, data FROM wallet_cache_v2
            WHERE chain_id = $1 AND seed_hash = ANY($2)
            "#,
		)
		.bind(chain_id.0.as_slice())
		.bind(&seed_hash_bytes)
		.fetch_all(&self.pool)
		.await
		{
			Ok(rows) => rows,
			Err(e) => {
				log::warn!("Failed to query wallet states: {e}");
				return seed_hashes.iter().map(|_| None).collect();
			},
		};

		// Build a lookup map: seed_hash -> CachedWalletState
		let mut found: std::collections::HashMap<H256, CachedWalletState> =
			std::collections::HashMap::new();
		for row in rows {
			let seed_hash_bytes: Vec<u8> = row.get("seed_hash");
			let data: Vec<u8> = row.get("data");

			if seed_hash_bytes.len() == 32 {
				let seed_hash = H256::from_slice(&seed_hash_bytes);
				match CachedWalletState::from_value_bytes(&data, seed_hash) {
					Ok(cached) => {
						found.insert(seed_hash, cached);
					},
					Err(e) => {
						log::warn!("Failed to decode wallet state: {e}");
					},
				}
			}
		}

		seed_hashes.iter().map(|h| found.remove(h)).collect()
	}

	async fn set_wallet_states(&self, chain_id: H256, wallets: &[CachedWalletState]) {
		if wallets.is_empty() {
			return;
		}

		let mut tx = match self.pool.begin().await {
			Ok(tx) => tx,
			Err(e) => {
				log::warn!("Failed to begin transaction for wallet states: {e}");
				return;
			},
		};

		for wallet in wallets {
			let encoded = match wallet.to_value_bytes() {
				Ok(b) => b,
				Err(e) => {
					log::warn!("Failed to serialize wallet state for {:?}: {e}", wallet.seed_hash);
					continue;
				},
			};

			if let Err(e) = sqlx::query(
				r#"
                INSERT INTO wallet_cache_v2 (chain_id, seed_hash, block_height, data, updated_at)
                VALUES ($1, $2, $3, $4, NOW())
                ON CONFLICT (chain_id, seed_hash)
                DO UPDATE SET block_height = EXCLUDED.block_height,
                              data = EXCLUDED.data,
                              updated_at = NOW()
                "#,
			)
			.bind(chain_id.0.as_slice())
			.bind(wallet.seed_hash.0.as_slice())
			.bind(wallet.block_height as i64)
			.bind(&encoded)
			.execute(&mut *tx)
			.await
			{
				log::warn!("Failed to set wallet state for {:?}: {e}", wallet.seed_hash);
				continue;
			}
		}

		if let Err(e) = tx.commit().await {
			log::warn!("Failed to commit wallet states: {e}");
			return;
		}

		log::info!("Saved {} wallet cache entries", wallets.len());
	}

	async fn delete_wallet_states(&self, chain_id: H256, seed_hashes: &[H256]) {
		if seed_hashes.is_empty() {
			return;
		}

		let seed_hash_bytes: Vec<Vec<u8>> = seed_hashes.iter().map(|h| h.0.to_vec()).collect();

		if let Err(e) = sqlx::query(
			r#"
            DELETE FROM wallet_cache_v2
            WHERE chain_id = $1 AND seed_hash = ANY($2)
            "#,
		)
		.bind(chain_id.0.as_slice())
		.bind(&seed_hash_bytes)
		.execute(&self.pool)
		.await
		{
			log::warn!("Failed to delete wallet states: {e}");
		}
	}

	async fn gc_ledger_snapshots(&self, chain_id: H256, keep_heights: &[u64]) {
		let keep_heights_i64: Vec<i64> = keep_heights.iter().map(|&h| h as i64).collect();

		match sqlx::query(
			r#"
            DELETE FROM ledger_snapshots_v2
            WHERE chain_id = $1 AND block_height <> ALL($2)
            "#,
		)
		.bind(chain_id.0.as_slice())
		.bind(&keep_heights_i64)
		.execute(&self.pool)
		.await
		{
			Ok(r) => {
				let count = r.rows_affected();
				if count > 0 {
					log::info!("GC: removed {} stale ledger snapshots", count);
				}
			},
			Err(e) => {
				log::warn!("Failed to GC ledger snapshots: {e}");
			},
		}
	}

	async fn get_all_cached_wallet_heights(&self, chain_id: H256) -> Vec<u64> {
		let rows: Vec<PgRow> = match sqlx::query(
			r#"
            SELECT DISTINCT block_height FROM wallet_cache_v2
            WHERE chain_id = $1
            "#,
		)
		.bind(chain_id.0.as_slice())
		.fetch_all(&self.pool)
		.await
		{
			Ok(rows) => rows,
			Err(e) => {
				log::warn!("Failed to query cached wallet heights: {e}");
				return Vec::new();
			},
		};

		rows.iter().map(|row| row.get::<i64, _>("block_height") as u64).collect()
	}
}

// =============================================================================
// Cache eviction methods
// =============================================================================

impl PostgresBackend {
	/// Evict wallet state cache entries older than the specified number of days.
	///
	/// Returns the number of entries evicted.
	pub async fn evict_stale_wallet_cache(&self, max_age_days: u32) -> u64 {
		let result = sqlx::query(
			r#"
            DELETE FROM wallet_cache_v2
            WHERE updated_at < NOW() - INTERVAL '1 day' * $1
            "#,
		)
		.bind(max_age_days as i32)
		.execute(&self.pool)
		.await;

		match result {
			Ok(r) => {
				let count = r.rows_affected();
				if count > 0 {
					log::info!(
						"Evicted {} stale wallet cache entries (older than {} days)",
						count,
						max_age_days
					);
				}
				count
			},
			Err(e) => {
				log::warn!("Failed to evict stale wallet cache entries: {e}");
				0
			},
		}
	}

	/// Get the count of wallet state cache entries.
	pub async fn wallet_cache_count(&self) -> u64 {
		let result: Option<(i64,)> = sqlx::query_as(r#"SELECT COUNT(*) FROM wallet_cache_v2"#)
			.fetch_optional(&self.pool)
			.await
			.ok()
			.flatten();

		result.map(|(count,)| count as u64).unwrap_or(0)
	}
}
