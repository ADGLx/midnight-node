// This file is part of midnight-node.
// Copyright (C) Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// You may not use this file except in compliance with the License.
// You may obtain a copy of the License at
// http://www.apache.org/licenses/LICENSE-2.0

//! Host-side parallel pre-verifier that wraps `BlockImport`.
//!
//! Substrate's import pipeline hands a block to the WASM runtime, which then
//! dispatches extrinsics serially; each `pallet_midnight::send_mn_transaction`
//! triggers a heavy `well_formed` ZK check inside the runtime. This wrapper
//! decodes the body up-front, runs `well_formed` for each midnight tx in
//! parallel on the host (rayon), and stuffs the results into the shared
//! `STRICT_TX_VALIDATION_CACHE` keyed by `(parent_block_hash, tx_hash)`. The
//! runtime's later serial dispatch hits the cache and skips the verify work.
//!
//! Correctness: pre-verify uses the *latest-observed* `LedgerState` as the
//! reference state (published by `Bridge::apply_transaction`). `well_formed`
//! only reads parameters / verifier keys from that state, so a stable prior
//! block suffices. On reorg we may cache under a different `ref_anchor`; the
//! runtime then misses and re-runs — no correctness impact.
//!
//! Wrappers (`utility::batch`, `scheduler`, `sudo`) are not yet walked — see
//! `memory/project_pre_verify_container_trait.md`.

use midnight_node_runtime::{
	RuntimeCall, TimestampCall, UncheckedExtrinsic,
	opaque::Block,
};
use parity_scale_codec::{Decode, Encode};
use sc_consensus::{BlockImport, BlockImportParams, ImportResult};
use sp_api::{Core, ProvideRuntimeApi};
use sp_runtime::traits::{Block as BlockT, Header as HeaderT};
use std::sync::Arc;

pub struct MidnightPreVerifier<Inner, Client> {
	inner: Inner,
	client: Arc<Client>,
}

impl<Inner, Client> MidnightPreVerifier<Inner, Client> {
	pub fn new(inner: Inner, client: Arc<Client>) -> Self {
		Self { inner, client }
	}
}

impl<Inner: Clone, Client> Clone for MidnightPreVerifier<Inner, Client> {
	fn clone(&self) -> Self {
		Self { inner: self.inner.clone(), client: self.client.clone() }
	}
}

/// Decode an opaque extrinsic into a typed one. Returns `None` on decode
/// failure — the runtime will diagnose those authoritatively.
fn decode_extrinsic(raw: &<Block as BlockT>::Extrinsic) -> Option<UncheckedExtrinsic> {
	let encoded = raw.encode();
	UncheckedExtrinsic::decode(&mut &encoded[..]).ok()
}

/// Collect every top-level `pallet_midnight::send_mn_transaction` payload from
/// a block body. Wrapper calls (`utility::batch`, `sudo`, scheduler) are not
/// walked yet.
fn collect_midnight_txs(body: &[<Block as BlockT>::Extrinsic]) -> Vec<Vec<u8>> {
	body.iter()
		.filter_map(decode_extrinsic)
		.filter_map(|ext| match ext.function {
			RuntimeCall::Midnight(pallet_midnight::Call::send_mn_transaction { midnight_tx }) =>
				Some(midnight_tx),
			_ => None,
		})
		.collect()
}

/// Read the timestamp inherent (ms since epoch). Substrate places the
/// timestamp `set` extrinsic at index 0; falls back to scanning if absent.
fn block_timestamp_millis(body: &[<Block as BlockT>::Extrinsic]) -> Option<u64> {
	body.iter().find_map(|raw| {
		let ext = decode_extrinsic(raw)?;
		match ext.function {
			RuntimeCall::Timestamp(TimestampCall::set { now }) => Some(now),
			_ => None,
		}
	})
}

#[async_trait::async_trait]
impl<Inner, Client> BlockImport<Block> for MidnightPreVerifier<Inner, Client>
where
	Inner: BlockImport<Block> + Send + Sync,
	Client: ProvideRuntimeApi<Block> + Send + Sync + 'static,
	Client::Api: Core<Block>,
{
	type Error = Inner::Error;

	async fn check_block(
		&self,
		block: sc_consensus::BlockCheckParams<Block>,
	) -> Result<ImportResult, Self::Error> {
		self.inner.check_block(block).await
	}

	async fn import_block(
		&self,
		block: BlockImportParams<Block>,
	) -> Result<ImportResult, Self::Error> {
		// Pre-verify only when we have the body. Stale / justification-only
		// imports have nothing to verify.
		if let Some(body) = &block.body {
			let midnight_txs = collect_midnight_txs(body);
			if !midnight_txs.is_empty() {
				let parent_hash: [u8; 32] = (*block.header.parent_hash()).into();
				let tblock_ms = block_timestamp_millis(body).unwrap_or(0);
				// Use the runtime version at the parent block, not the
				// binary's compile-time const. Mainnet has been through ~14
				// runtime upgrades and cache keys include `spec_version`, so
				// using the const causes a MISS for every historical block.
				if let Ok(rv) = self.client.runtime_api().version(*block.header.parent_hash()) {
					let runtime_version = rv.spec_version;
					// Register the in-flight marker BEFORE spawning so any
					// concurrent `get_verified_transaction` from the runtime
					// sees a pending state and waits for our result.
					let in_flight =
						midnight_node_ledger::ledger_8::register_in_flight_prevalidation(
							parent_hash,
						);
					std::thread::spawn(move || {
						let slices: Vec<&[u8]> =
							midnight_txs.iter().map(|v| v.as_slice()).collect();
						midnight_node_ledger::host_api::ledger_8::prevalidate_block(
							parent_hash,
							tblock_ms,
							runtime_version,
							&slices,
						);
						in_flight.mark_done();
					});
				}
			}
		}

		// Hand off to the inner BlockImport (grandpa, etc.) — runs in
		// parallel with the prevalidate thread spawned above.
		self.inner.import_block(block).await
	}
}
