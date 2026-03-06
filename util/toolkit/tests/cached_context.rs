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

//! Integration tests verifying `build_fork_aware_context_cached` produces the
//! same result as `build_fork_aware_context_raw` across all cache scenarios.

mod common;

use common::test_image;
use midnight_node_ledger_helpers::{
	DefaultDB, LedgerContext, WalletSeed, serialize, serialize_untagged,
};
use midnight_node_toolkit::{
	fetcher::fetch_storage::{
		WalletStateCaching, file_backend::FileBackend, postgres_backend::PostgresBackend,
		redb_backend::RedbBackend,
	},
	serde_def::SourceTransactions,
	tx_generator::{
		builder::{build_fork_aware_context_cached, build_fork_aware_context_raw},
		source::GetTxsFromFile,
	},
};
use testcontainers::{GenericImage, ImageExt, core::WaitFor, runners::AsyncRunner};
use tokio::sync::OnceCell;

// ---------------------------------------------------------------------------
// Genesis helpers
// ---------------------------------------------------------------------------

fn load_genesis_source() -> SourceTransactions {
	let genesis_path =
		format!("{}/test-data/genesis/genesis_block_undeployed.mn", env!("CARGO_MANIFEST_DIR"));
	let batches = GetTxsFromFile::load_single_or_multiple(&genesis_path)
		.expect("failed to load genesis file");
	SourceTransactions::from_batches(batches.batches, true)
}

/// Assign sequential block numbers and deterministic hashes so `chain_id()`
/// returns `Some` (it looks for a block with `number == 1`).
fn assign_block_numbers(source: &mut SourceTransactions) {
	for (i, block) in source.blocks.iter_mut().enumerate() {
		block.number = i as u64;
		block.hash = {
			let mut h = [0u8; 32];
			h[..8].copy_from_slice(&(i as u64).to_le_bytes());
			h
		};
	}
}

fn wallet_seed(hex_byte: u8) -> WalletSeed {
	let hex = format!("{:0>64}", format!("{:02x}", hex_byte));
	WalletSeed::try_from_hex_str(&hex).unwrap()
}

// ---------------------------------------------------------------------------
// Context comparison
// ---------------------------------------------------------------------------

fn assert_contexts_equal(
	label: &str,
	cached: &LedgerContext<DefaultDB>,
	raw: &LedgerContext<DefaultDB>,
	seeds: &[WalletSeed],
) {
	// Compare ledger state
	let cached_bytes = {
		let state = cached.ledger_state.lock().unwrap();
		serialize(&**state).expect("serialize cached ledger state")
	};
	let raw_bytes = {
		let state = raw.ledger_state.lock().unwrap();
		serialize(&**state).expect("serialize raw ledger state")
	};
	assert_eq!(cached_bytes, raw_bytes, "{label}: ledger state diverged");

	// Compare per-wallet state
	let cached_wallets = cached.wallets.lock().unwrap();
	let raw_wallets = raw.wallets.lock().unwrap();
	assert_eq!(cached_wallets.len(), raw_wallets.len(), "{label}: wallet count mismatch");

	for seed in seeds {
		let cw = cached_wallets
			.get(seed)
			.unwrap_or_else(|| panic!("{label}: cached wallet missing"));
		let rw = raw_wallets.get(seed).unwrap_or_else(|| panic!("{label}: raw wallet missing"));

		let cs = serialize_untagged(&cw.shielded.state).expect("serialize cached shielded");
		let rs = serialize_untagged(&rw.shielded.state).expect("serialize raw shielded");
		assert_eq!(cs, rs, "{label}: shielded state diverged for seed {seed:?}");

		let cd = cw
			.dust
			.dust_local_state
			.as_ref()
			.map(|s| serialize_untagged(&**s).expect("serialize cached dust"));
		let rd = rw
			.dust
			.dust_local_state
			.as_ref()
			.map(|s| serialize_untagged(&**s).expect("serialize raw dust"));
		assert_eq!(cd, rd, "{label}: dust state diverged for seed {seed:?}");
	}
}

// ---------------------------------------------------------------------------
// Test scenarios (backend-agnostic)
// ---------------------------------------------------------------------------

/// Test 1: No wallet seeds — early-return to raw. Compare ledger state only.
async fn test_no_seeds(backend: &dyn WalletStateCaching) {
	let mut source = load_genesis_source();
	assign_block_numbers(&mut source);

	let cached_ctx = build_fork_aware_context_cached(&[], &source, Some(backend)).await;
	let raw_ctx = build_fork_aware_context_raw(&source, &[]);

	let cached = cached_ctx.into_ledger8().expect("cached: expected ledger 8");
	let raw = raw_ctx.into_ledger8().expect("raw: expected ledger 8");

	assert_contexts_equal("no_seeds", &cached, &raw, &[]);
}

/// Test 2: All uncached (Case A) — 2 wallet seeds, empty DB.
async fn test_all_uncached(backend: &dyn WalletStateCaching) {
	let mut source = load_genesis_source();
	assign_block_numbers(&mut source);
	let seeds = vec![wallet_seed(0x01), wallet_seed(0x02)];

	let cached_ctx = build_fork_aware_context_cached(&seeds, &source, Some(backend)).await;
	let raw_ctx = build_fork_aware_context_raw(&source, &seeds);

	let cached = cached_ctx.into_ledger8().expect("cached: expected ledger 8");
	let raw = raw_ctx.into_ledger8().expect("raw: expected ledger 8");

	assert_contexts_equal("all_uncached", &cached, &raw, &seeds);
}

/// Test 3: All cached (Case B) — populate cache, then restore from it.
async fn test_all_cached(backend: &dyn WalletStateCaching) {
	let mut source = load_genesis_source();
	assign_block_numbers(&mut source);
	let seeds = vec![wallet_seed(0x01), wallet_seed(0x02)];

	// First call populates the cache (Case A internally)
	let _ = build_fork_aware_context_cached(&seeds, &source, Some(backend)).await;

	// Second call restores from cache (Case B)
	let cached_ctx = build_fork_aware_context_cached(&seeds, &source, Some(backend)).await;
	let raw_ctx = build_fork_aware_context_raw(&source, &seeds);

	let cached = cached_ctx.into_ledger8().expect("cached: expected ledger 8");
	let raw = raw_ctx.into_ledger8().expect("raw: expected ledger 8");

	assert_contexts_equal("all_cached", &cached, &raw, &seeds);
}

/// Test 4: Split — some cached, some new (Case C).
async fn test_split_cached(backend: &dyn WalletStateCaching) {
	let mut source = load_genesis_source();
	assign_block_numbers(&mut source);

	// First call: populate cache with only seed 0x01
	let seed1 = vec![wallet_seed(0x01)];
	let _ = build_fork_aware_context_cached(&seed1, &source, Some(backend)).await;

	// Second call: request [0x01, 0x02] — snapshot exists, 0x02 is new (Case C)
	let seeds = vec![wallet_seed(0x01), wallet_seed(0x02)];
	let cached_ctx = build_fork_aware_context_cached(&seeds, &source, Some(backend)).await;
	let raw_ctx = build_fork_aware_context_raw(&source, &seeds);

	let cached = cached_ctx.into_ledger8().expect("cached: expected ledger 8");
	let raw = raw_ctx.into_ledger8().expect("raw: expected ledger 8");

	assert_contexts_equal("split_cached", &cached, &raw, &seeds);
}

// ---------------------------------------------------------------------------
// Redb backend
// ---------------------------------------------------------------------------

#[tokio::test]
async fn redb_cached_context() {
	let tmp = tempfile::NamedTempFile::new().expect("failed to create temp file");
	let backend = RedbBackend::new(tmp.path());

	test_no_seeds(&backend).await;

	// Each scenario needs a fresh backend to avoid cache state leaking between tests
	let tmp2 = tempfile::NamedTempFile::new().expect("failed to create temp file");
	let backend2 = RedbBackend::new(tmp2.path());
	test_all_uncached(&backend2).await;

	let tmp3 = tempfile::NamedTempFile::new().expect("failed to create temp file");
	let backend3 = RedbBackend::new(tmp3.path());
	test_all_cached(&backend3).await;

	let tmp4 = tempfile::NamedTempFile::new().expect("failed to create temp file");
	let backend4 = RedbBackend::new(tmp4.path());
	test_split_cached(&backend4).await;
}

// ---------------------------------------------------------------------------
// Postgres backend
// ---------------------------------------------------------------------------

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

/// Each scenario uses a distinct chain_id (via unique block hashes) to isolate
/// cache state without needing separate databases.
async fn postgres_with_unique_source() -> (PostgresBackend, SourceTransactions) {
	let backend = PostgresBackend::new(postgres_url().await).await;
	let mut source = load_genesis_source();
	assign_block_numbers(&mut source);

	// Make block hashes unique per call to avoid cache collisions across tests
	let salt: [u8; 8] = rand::random();
	for block in &mut source.blocks {
		block.hash[24..].copy_from_slice(&salt);
	}

	(backend, source)
}

#[tokio::test]
async fn postgres_cached_context_no_seeds() {
	let (backend, source) = postgres_with_unique_source().await;

	let cached_ctx = build_fork_aware_context_cached(&[], &source, Some(&backend)).await;
	let raw_ctx = build_fork_aware_context_raw(&source, &[]);

	let cached = cached_ctx.into_ledger8().expect("cached: expected ledger 8");
	let raw = raw_ctx.into_ledger8().expect("raw: expected ledger 8");

	assert_contexts_equal("pg_no_seeds", &cached, &raw, &[]);
}

#[tokio::test]
async fn postgres_cached_context_all_uncached() {
	let (backend, source) = postgres_with_unique_source().await;
	let seeds = vec![wallet_seed(0x01), wallet_seed(0x02)];

	let cached_ctx = build_fork_aware_context_cached(&seeds, &source, Some(&backend)).await;
	let raw_ctx = build_fork_aware_context_raw(&source, &seeds);

	let cached = cached_ctx.into_ledger8().expect("cached: expected ledger 8");
	let raw = raw_ctx.into_ledger8().expect("raw: expected ledger 8");

	assert_contexts_equal("pg_all_uncached", &cached, &raw, &seeds);
}

#[tokio::test]
async fn postgres_cached_context_all_cached() {
	let (backend, source) = postgres_with_unique_source().await;
	let seeds = vec![wallet_seed(0x01), wallet_seed(0x02)];

	let _ = build_fork_aware_context_cached(&seeds, &source, Some(&backend)).await;
	let cached_ctx = build_fork_aware_context_cached(&seeds, &source, Some(&backend)).await;
	let raw_ctx = build_fork_aware_context_raw(&source, &seeds);

	let cached = cached_ctx.into_ledger8().expect("cached: expected ledger 8");
	let raw = raw_ctx.into_ledger8().expect("raw: expected ledger 8");

	assert_contexts_equal("pg_all_cached", &cached, &raw, &seeds);
}

#[tokio::test]
async fn postgres_cached_context_split() {
	let (backend, source) = postgres_with_unique_source().await;

	let seed1 = vec![wallet_seed(0x01)];
	let _ = build_fork_aware_context_cached(&seed1, &source, Some(&backend)).await;

	let seeds = vec![wallet_seed(0x01), wallet_seed(0x02)];
	let cached_ctx = build_fork_aware_context_cached(&seeds, &source, Some(&backend)).await;
	let raw_ctx = build_fork_aware_context_raw(&source, &seeds);

	let cached = cached_ctx.into_ledger8().expect("cached: expected ledger 8");
	let raw = raw_ctx.into_ledger8().expect("raw: expected ledger 8");

	assert_contexts_equal("pg_split_cached", &cached, &raw, &seeds);
}

// ---------------------------------------------------------------------------
// File backend
// ---------------------------------------------------------------------------

#[tokio::test]
async fn file_cached_context() {
	let tmp = tempfile::TempDir::new().expect("failed to create temp dir");
	let backend = FileBackend::new(tmp.path());
	test_no_seeds(&backend).await;

	let tmp2 = tempfile::TempDir::new().expect("failed to create temp dir");
	let backend2 = FileBackend::new(tmp2.path());
	test_all_uncached(&backend2).await;

	let tmp3 = tempfile::TempDir::new().expect("failed to create temp dir");
	let backend3 = FileBackend::new(tmp3.path());
	test_all_cached(&backend3).await;

	let tmp4 = tempfile::TempDir::new().expect("failed to create temp dir");
	let backend4 = FileBackend::new(tmp4.path());
	test_split_cached(&backend4).await;
}
