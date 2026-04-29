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

//! Bulk-read cNIGHT observation data source.
//!
//! At startup, run the four observation queries against db-sync across
//! `[0, current_cardano_tip]` and hold the result in memory. Bulk observation
//! queries thereafter come from the in-memory cache; a sliding-window refresh
//! extends the cache as the chain advances. Queries past the current horizon
//! delegate to a live db-backed source so the node keeps importing.
//!
//! Trade vs. an on-disk snapshot file: pay ~2 min of postgres work per node
//! start instead of carrying multi-MB binaries in the repo.

use crate::data_source::candidates_data_source::observed_async_trait;
use crate::data_source::cnight_observation::MidnightCNightObservationDataSourceImpl;
use crate::data_source::metrics::MidnightDataSourceMetrics;
use crate::db::MultiAssetCache;
use crate::{MidnightCNightObservationDataSource, ObservedUtxo};
use cardano_serialization_lib::{Address, EnterpriseAddress};
use midnight_primitives_cnight_observation::{CNightAddresses, CardanoPosition, ObservedUtxos};
use sidechain_domain::McBlockHash;
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Effectively-no-limit page size for bulk pulls. The query path supports
/// `LIMIT` for paged use but the sliding window wants the whole range in
/// one shot.
const LARGE_LIMIT: usize = 5_000_000;

/// If the resolved tip comes within this many cardano blocks of the snapshot's
/// `end_block`, kick off an async refresh to extend the in-memory window.
const REFRESH_THRESHOLD: u32 = 10_000;

/// How many cardano blocks past the requested target to leave un-fetched
/// (re-org safety). The follower never asks for queries past the stable
/// horizon anyway, so this is mostly belt-and-braces.
const REFRESH_STABILITY_MARGIN: u32 = 2_170;

/// Pull every cnight observation event in `[from_block, to_block]` (inclusive)
/// and return them sorted ascending by `tx_position`.
///
/// Used at startup (`from_block = 0`) and by the sliding-window refresh
/// (`from_block = old_end + 1`).
pub async fn bulk_pull(
	pool: &PgPool,
	cfg: &CNightAddresses,
	from_block: u32,
	to_block: u32,
) -> Result<Vec<ObservedUtxo>, Box<dyn std::error::Error + Send + Sync>> {
	let ds = MidnightCNightObservationDataSourceImpl::new(pool.clone(), None, 0);

	let mapping_validator_address = Address::from_bech32(&cfg.mapping_validator_address)
		.map_err(|e| format!("invalid mapping validator address: {e}"))?;
	let cardano_network =
		mapping_validator_address.network_id().map_err(|e| format!("network_id: {e}"))?;
	let mapping_validator_policy_id = EnterpriseAddress::from_address(&mapping_validator_address)
		.ok_or("mapping validator address is not EnterpriseAddress")?
		.payment_cred()
		.to_scripthash()
		.ok_or("mapping validator address has no script hash")?;

	let asset_cache = MultiAssetCache::new(pool.clone());
	let auth_token_ident = asset_cache
		.resolve_ident(
			&mapping_validator_policy_id.to_bytes(),
			cfg.auth_token_asset_name.as_bytes(),
		)
		.await?;
	let cnight_ident = asset_cache
		.resolve_ident(&cfg.cnight_policy_id, cfg.cnight_asset_name.as_bytes())
		.await?;

	let start = CardanoPosition {
		block_hash: McBlockHash([0u8; 32]),
		block_number: from_block,
		block_timestamp: Default::default(),
		tx_index_in_block: 0,
	};
	let end = CardanoPosition {
		block_hash: McBlockHash([0u8; 32]),
		block_number: to_block,
		block_timestamp: Default::default(),
		tx_index_in_block: u32::MAX,
	};

	let (low_bounds, high_bounds) = tokio::try_join!(
		crate::db::get_low_bounds(pool, from_block.into()),
		crate::db::get_high_bounds(pool, to_block.into()),
	)?;
	let low_bounds =
		low_bounds.ok_or_else(|| format!("get_low_bounds({from_block}) returned None"))?;
	let high_bounds =
		high_bounds.ok_or_else(|| format!("get_high_bounds({to_block}) returned None"))?;

	let paged = crate::db::PagedQuery {
		start: &start,
		end: &end,
		limit: LARGE_LIMIT,
		offset: 0,
		low_bound: low_bounds,
		high_bound: high_bounds,
	};

	let mut all = Vec::new();
	if let Some(ident) = auth_token_ident {
		all.extend(
			ds.get_registration_utxos(
				cardano_network,
				ident,
				&cfg.mapping_validator_address,
				&paged,
			)
			.await?,
		);
	}
	all.extend(
		ds.get_deregistration_utxos(cardano_network, &cfg.mapping_validator_address, &paged)
			.await?,
	);
	if let Some(ident) = cnight_ident {
		all.extend(ds.get_asset_create_utxos(cardano_network, ident, &paged).await?);
		all.extend(ds.get_asset_spend_utxos(cardano_network, ident, &paged).await?);
	}
	all.sort();
	Ok(all)
}

/// Cached result of the previous `get_utxos_up_to_capacity` call. During
/// initial sync many consecutive Midnight blocks share the same Cardano tip,
/// so recomputing the window each time is wasted work.
#[derive(Clone)]
struct LastObservation {
	start_position: CardanoPosition,
	current_tip: McBlockHash,
	result: ObservedUtxos,
	/// True iff the previous call returned all data up to the requested `end`
	/// (i.e. not truncated by `tx_capacity`). Only full-window results can be
	/// sliced when `start_position` advances.
	full_window: bool,
}

/// A `MidnightCNightObservationDataSource` backed by an in-memory event vector
/// built once at startup, with an async sliding-window refresh and a live
/// db-backed fallback for queries past the current horizon.
pub struct BulkCachedCNightObservationDataSource {
	/// Sorted events. `RwLock<Arc<...>>` lets the refresh task swap in an
	/// extended vec without blocking concurrent readers.
	all_events: Arc<std::sync::RwLock<Arc<Vec<ObservedUtxo>>>>,
	/// Used exclusively for `get_block_by_hash` — a single indexed lookup
	/// per call when the block is not yet in `block_position_cache`.
	pool: PgPool,
	/// Memoizes `current_tip` (cardano block hash) → `CardanoPosition`. Many
	/// consecutive midnight blocks share the same Cardano tip during sync,
	/// so without this every call would do a postgres round-trip.
	block_position_cache: Arc<Mutex<HashMap<McBlockHash, CardanoPosition>>>,
	last_observation: Arc<Mutex<Option<LastObservation>>>,
	/// Largest cardano block number for which we have events. Queries whose
	/// resolved tip goes past this delegate to `db_fallback` AND trigger an
	/// async refresh.
	snapshot_end_block: Arc<std::sync::RwLock<Option<u32>>>,
	db_fallback: Arc<MidnightCNightObservationDataSourceImpl>,
	/// cNIGHT addresses cached so the sliding-window refresh can re-run the
	/// observation queries without re-reading the chainspec JSON.
	cnight_addresses: CNightAddresses,
	/// Single-flight gate for sliding-window refreshes.
	refresh_in_flight: Arc<Mutex<bool>>,
	#[allow(dead_code)]
	metrics_opt: Option<MidnightDataSourceMetrics>,
}

impl BulkCachedCNightObservationDataSource {
	pub fn new(
		events: Vec<ObservedUtxo>,
		pool: PgPool,
		db_fallback: Arc<MidnightCNightObservationDataSourceImpl>,
		cnight_addresses: CNightAddresses,
		metrics_opt: Option<MidnightDataSourceMetrics>,
	) -> Self {
		let snapshot_end_block = events.last().map(|e| e.header.tx_position.block_number);
		Self {
			all_events: Arc::new(std::sync::RwLock::new(Arc::new(events))),
			pool,
			block_position_cache: Arc::new(Mutex::new(HashMap::new())),
			last_observation: Arc::new(Mutex::new(None)),
			snapshot_end_block: Arc::new(std::sync::RwLock::new(snapshot_end_block)),
			db_fallback,
			cnight_addresses,
			refresh_in_flight: Arc::new(Mutex::new(false)),
			metrics_opt,
		}
	}

	/// Trigger an async sliding-window refresh if not already in flight.
	/// Returns immediately. Single-flight: concurrent triggers are no-ops.
	fn maybe_kick_refresh(&self, target_end: u32) {
		{
			let mut g = match self.refresh_in_flight.lock() {
				Ok(g) => g,
				Err(_) => return,
			};
			if *g {
				return;
			}
			*g = true;
		}

		let pool = self.pool.clone();
		let cfg = self.cnight_addresses.clone();
		let all_events = Arc::clone(&self.all_events);
		let snapshot_end_block = Arc::clone(&self.snapshot_end_block);
		let in_flight = Arc::clone(&self.refresh_in_flight);

		tokio::spawn(async move {
			if let Err(e) =
				refresh_window(&pool, &cfg, &all_events, &snapshot_end_block, target_end).await
			{
				log::warn!(
					target: "cnight::sliding-window",
					"refresh failed (ignored, db_fallback continues to serve): {e}"
				);
			}
			if let Ok(mut g) = in_flight.lock() {
				*g = false;
			}
		});
	}
}

/// Pull events in `(old_end, target_end]` and atomically extend the shared
/// events vec. New events sort strictly after every existing event, so we
/// don't need a global re-sort.
async fn refresh_window(
	pool: &PgPool,
	cfg: &CNightAddresses,
	all_events: &Arc<std::sync::RwLock<Arc<Vec<ObservedUtxo>>>>,
	snapshot_end_block: &Arc<std::sync::RwLock<Option<u32>>>,
	target_end: u32,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
	let old_end = snapshot_end_block
		.read()
		.map_err(|e| format!("snapshot_end_block read poisoned: {e}"))?
		.unwrap_or(0);
	if target_end <= old_end {
		return Ok(());
	}
	let from_block = old_end.saturating_add(1);
	log::info!(
		target: "cnight::sliding-window",
		"refresh kicked off: extending window [{from_block}, {target_end}] (old end_block={old_end})"
	);
	let t0 = std::time::Instant::now();
	let extension = bulk_pull(pool, cfg, from_block, target_end).await?;
	{
		let mut events_guard =
			all_events.write().map_err(|e| format!("all_events write poisoned: {e}"))?;
		let mut new_vec: Vec<ObservedUtxo> =
			Vec::with_capacity(events_guard.len() + extension.len());
		new_vec.extend_from_slice(events_guard.as_slice());
		new_vec.extend(extension);
		*events_guard = Arc::new(new_vec);
	}
	*snapshot_end_block
		.write()
		.map_err(|e| format!("snapshot_end_block write poisoned: {e}"))? = Some(target_end);
	log::info!(
		target: "cnight::sliding-window",
		"refresh done: end_block now {target_end} (took {:?})",
		t0.elapsed()
	);
	Ok(())
}

/// From a sorted vec, return the slice `[a..b)` covering events whose
/// `tx_position` falls in `[start, end)`.
fn slice_range<'a>(
	vec: &'a [ObservedUtxo],
	start: &CardanoPosition,
	end: &CardanoPosition,
) -> &'a [ObservedUtxo] {
	let a = vec.partition_point(|u| u.header.tx_position < *start);
	let b = vec.partition_point(|u| u.header.tx_position < *end);
	&vec[a..b]
}

observed_async_trait!(
impl MidnightCNightObservationDataSource for BulkCachedCNightObservationDataSource {
	async fn get_utxos_up_to_capacity(
		&self,
		config: &CNightAddresses,
		start_position: &CardanoPosition,
		current_tip: McBlockHash,
		tx_capacity: usize,
	) -> Result<ObservedUtxos, Box<dyn std::error::Error + Send + Sync>> {
		// Same-tip cache: if `current_tip` hasn't advanced, the Cardano window
		// hasn't grown. Reuse the previous result directly (exact match) or
		// filter it to `>= start_position` (advanced within a full window —
		// only safe when the prior call wasn't truncated).
		if let Ok(guard) = self.last_observation.lock()
			&& let Some(last) = guard.as_ref()
			&& last.current_tip == current_tip
		{
			if last.start_position == *start_position {
				return Ok(last.result.clone());
			} else if last.full_window
				&& *start_position >= last.start_position
				&& *start_position <= last.result.end
			{
				let filtered: Vec<_> = last
					.result
					.utxos
					.iter()
					.filter(|u| u.header.tx_position >= *start_position)
					.cloned()
					.collect();
				return Ok(ObservedUtxos {
					start: start_position.clone(),
					end: last.result.end.clone(),
					utxos: filtered,
				});
			}
		}

		// Resolve `current_tip` (cardano block hash) → CardanoPosition.
		let cached = self
			.block_position_cache
			.lock()
			.ok()
			.and_then(|g| g.get(&current_tip).cloned());
		let tip_pos: CardanoPosition = match cached {
			Some(pos) => pos,
			None => {
				let block = crate::db::get_block_by_hash(&self.pool, current_tip.clone())
					.await?
					.ok_or_else(|| format!("missing block for tip {:?}", current_tip))?;
				let pos: CardanoPosition = block.into();
				if let Ok(mut guard) = self.block_position_cache.lock() {
					guard.insert(current_tip.clone(), pos.clone());
				}
				pos
			},
		};

		// Sliding-window refresh + horizon delegation.
		let horizon_opt = self.snapshot_end_block.read().ok().and_then(|g| *g);
		if let Some(horizon) = horizon_opt {
			if tip_pos.block_number.saturating_add(REFRESH_THRESHOLD) >= horizon {
				let target_end = tip_pos
					.block_number
					.saturating_add(REFRESH_THRESHOLD)
					.saturating_add(REFRESH_STABILITY_MARGIN);
				self.maybe_kick_refresh(target_end);
			}
			if tip_pos.block_number > horizon {
				log::debug!(
					"cNIGHT observation: tip cardano block {} past snapshot horizon {}, delegating to DB",
					tip_pos.block_number, horizon,
				);
				return self
					.db_fallback
					.get_utxos_up_to_capacity(config, start_position, current_tip, tx_capacity)
					.await;
			}
		}

		let end = tip_pos.increment();
		// Snapshot the current Arc<Vec> so a concurrent refresh that swaps in
		// a new Arc doesn't disturb our local clone.
		let events_snapshot: Arc<Vec<ObservedUtxo>> = self
			.all_events
			.read()
			.map(|g| Arc::clone(&g))
			.unwrap_or_else(|_| Arc::new(Vec::new()));
		let window = slice_range(&events_snapshot, start_position, &end);

		// Truncate to `tx_capacity` whole transactions.
		let mut truncated: Vec<ObservedUtxo> = Vec::with_capacity(window.len());
		let mut num_txs: usize = 0;
		let mut cur_tx: Option<CardanoPosition> = None;
		for utxo in window {
			if cur_tx.as_ref().is_none_or(|tx| tx < &utxo.header.tx_position) {
				num_txs += 1;
				cur_tx = Some(utxo.header.tx_position.clone());
			}
			if num_txs == tx_capacity {
				break;
			}
			truncated.push(utxo.clone());
		}

		let full_window = num_txs < tx_capacity;
		let result_end = if full_window {
			end
		} else {
			truncated
				.last()
				.map(|u| u.header.tx_position.clone())
				.unwrap_or_else(|| start_position.clone())
				.increment()
		};

		let result = ObservedUtxos {
			start: start_position.clone(),
			end: result_end,
			utxos: truncated,
		};

		if let Ok(mut guard) = self.last_observation.lock() {
			*guard = Some(LastObservation {
				start_position: start_position.clone(),
				current_tip: current_tip.clone(),
				result: result.clone(),
				full_window,
			});
		}

		Ok(result)
	}
}
);
