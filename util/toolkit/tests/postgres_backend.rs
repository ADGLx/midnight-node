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

mod common;

use common::test_image;
use midnight_node_toolkit::fetcher::{
	fetch_storage::{WalletStateCaching, postgres_backend::PostgresBackend},
	wallet_state_cache::{CachedWalletState, LedgerSnapshot, SerializableBlockContext},
};
use subxt::utils::H256;
use testcontainers::{GenericImage, ImageExt, core::WaitFor, runners::AsyncRunner};
use tokio::sync::OnceCell;

struct SharedPostgres {
	_container: testcontainers::ContainerAsync<GenericImage>,
	url: String,
}

static POSTGRES: OnceCell<SharedPostgres> = OnceCell::const_new();

async fn postgres_url() -> &'static str {
	&POSTGRES
		.get_or_init(|| async {
			let (name, tag) = test_image("postgres");
			let password: String =
				(0..32).map(|_| format!("{:02x}", rand::random::<u8>())).collect();
			let container = GenericImage::new(name, tag)
				.with_wait_for(WaitFor::message_on_stderr(
					"database system is ready to accept connections",
				))
				.with_env_var("POSTGRES_PASSWORD", &password)
				.with_env_var("POSTGRES_USER", "postgres")
				.with_env_var("POSTGRES_DB", "toolkit_test")
				.start()
				.await
				.expect("failed to start postgres container");

			let port =
				container.get_host_port_ipv4(5432).await.expect("failed to get postgres port");
			let url = format!("postgres://postgres:{password}@localhost:{port}/toolkit_test");
			SharedPostgres { _container: container, url }
		})
		.await
		.url
}

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
async fn test_postgres_ledger_snapshot_roundtrip() {
	let backend = PostgresBackend::new(postgres_url().await).await;

	let chain_id = H256::from([100u8; 32]);

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
async fn test_postgres_wallet_states_batch() {
	let backend = PostgresBackend::new(postgres_url().await).await;

	let chain_id = H256::from([101u8; 32]);
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
	WalletStateCaching::set_wallet_states(&backend, chain_id, &[wallet_1, wallet_2]).await;

	// Batch retrieve
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
async fn test_postgres_latest_ledger_height() {
	let backend = PostgresBackend::new(postgres_url().await).await;

	let chain_id = H256::from([102u8; 32]);

	assert!(WalletStateCaching::get_latest_ledger_height(&backend, chain_id).await.is_none());

	WalletStateCaching::set_ledger_snapshot(&backend, chain_id, create_test_ledger_snapshot(100))
		.await;
	WalletStateCaching::set_ledger_snapshot(&backend, chain_id, create_test_ledger_snapshot(200))
		.await;

	let height = WalletStateCaching::get_latest_ledger_height(&backend, chain_id).await;
	assert_eq!(height, Some(200));
}

#[tokio::test]
async fn test_postgres_gc_ledger_snapshots() {
	let backend = PostgresBackend::new(postgres_url().await).await;

	let chain_id = H256::from([103u8; 32]);

	for h in [100, 200, 300] {
		WalletStateCaching::set_ledger_snapshot(&backend, chain_id, create_test_ledger_snapshot(h))
			.await;
	}

	// Keep only 200
	WalletStateCaching::gc_ledger_snapshots(&backend, chain_id, &[200]).await;

	assert!(WalletStateCaching::get_ledger_snapshot(&backend, chain_id, 100).await.is_none());
	assert!(WalletStateCaching::get_ledger_snapshot(&backend, chain_id, 200).await.is_some());
	assert!(WalletStateCaching::get_ledger_snapshot(&backend, chain_id, 300).await.is_none());
}

#[tokio::test]
async fn test_postgres_evict_stale_entries() {
	let backend = PostgresBackend::new(postgres_url().await).await;

	let chain_id = H256::from([104u8; 32]);
	let seed_hash = H256::from([2u8; 32]);

	// Save a wallet cache entry
	let wallet = create_test_wallet_state(seed_hash, 100);
	WalletStateCaching::set_wallet_states(&backend, chain_id, &[wallet]).await;

	// Evict entries older than 30 days (should not evict our fresh entry)
	let evicted = backend.evict_stale_wallet_cache(30).await;
	assert_eq!(evicted, 0);

	// Entry should still exist
	let results = WalletStateCaching::get_wallet_states(&backend, chain_id, &[seed_hash]).await;
	assert!(results[0].is_some());

	// Evict entries older than 0 days (should evict everything)
	let evicted = backend.evict_stale_wallet_cache(0).await;
	assert!(evicted >= 1);

	// Entry should be gone
	let results = WalletStateCaching::get_wallet_states(&backend, chain_id, &[seed_hash]).await;
	assert!(results[0].is_none());
}

#[tokio::test]
async fn test_postgres_delete_wallet_states() {
	let backend = PostgresBackend::new(postgres_url().await).await;

	let chain_id = H256::from([105u8; 32]);
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
