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
//! At startup the node runs the four observation queries against db-sync
//! across `[0, current_cardano_tip − security_param]` and holds the result
//! in memory. Bulk observation queries thereafter come from the in-memory
//! cache; queries past the bulk-read horizon delegate to a live db-backed
//! source so the node keeps importing as the chain advances.
//!
//! Trade vs. an on-disk snapshot file: pay ~2 min of postgres work per
//! node start (one bulk read) instead of carrying multi-MB snapshot
//! binaries in the repo and chasing the staleness/regen workflow.

use crate::data_source::candidates_data_source::observed_async_trait;
use crate::data_source::cnight_observation::MidnightCNightObservationDataSourceImpl;
use crate::db::MultiAssetCache;
use crate::{
	CreateData, DeregistrationData, MidnightCNightObservationDataSource, ObservedUtxo,
	ObservedUtxoData, ObservedUtxoHeader, RegistrationData, SpendData, UtxoIndexInTx,
};
use cardano_serialization_lib::{Address, EnterpriseAddress};
use midnight_primitives_cnight_observation::{CNightAddresses, CardanoPosition, ObservedUtxos};
use crate::data_source::metrics::MidnightDataSourceMetrics;
use sidechain_domain::McBlockHash;
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Sorted cNIGHT observation events, grouped by kind. Built by
/// `CNightObservationSnapshot::generate`; consumed by
/// `BulkCachedCNightObservationDataSource::new`.
#[derive(Clone, Debug, Default)]
pub struct CNightObservationSnapshot {
	pub registrations: Vec<ObservedUtxo>,
	pub deregistrations: Vec<ObservedUtxo>,
	pub creates: Vec<ObservedUtxo>,
	pub spends: Vec<ObservedUtxo>,
}

impl CNightObservationSnapshot {
	/// Total event count across all four vecs — handy for diagnostic logging.
	pub fn total_events(&self) -> usize {
		self.registrations.len()
			+ self.deregistrations.len()
			+ self.creates.len()
			+ self.spends.len()
	}

	/// Run the four observation queries across `[0, end_block_no]` and
	/// collect every event into a single in-memory snapshot. Each result
	/// vec is sorted ascending by `header.tx_position`.
	pub async fn generate(
		pool: PgPool,
		config: &CNightAddresses,
		end_block_no: u32,
	) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
		let ds = MidnightCNightObservationDataSourceImpl::new(pool.clone(), None, 0);

		let mapping_validator_address = Address::from_bech32(&config.mapping_validator_address)
			.map_err(|e| format!("invalid mapping validator address: {e}"))?;
		let cardano_network =
			mapping_validator_address.network_id().map_err(|e| format!("network_id: {e}"))?;
		let mapping_validator_policy_id =
			EnterpriseAddress::from_address(&mapping_validator_address)
				.ok_or("mapping validator address is not EnterpriseAddress")?
				.payment_cred()
				.to_scripthash()
				.ok_or("mapping validator address has no script hash")?;

		let asset_cache = MultiAssetCache::new(pool.clone());
		let auth_token_ident = asset_cache
			.resolve_ident(
				&mapping_validator_policy_id.to_bytes(),
				config.auth_token_asset_name.as_bytes(),
			)
			.await?;
		let cnight_ident = asset_cache
			.resolve_ident(&config.cnight_policy_id, config.cnight_asset_name.as_bytes())
			.await?;

		let start = CardanoPosition {
			block_hash: McBlockHash([0u8; 32]),
			block_number: 0,
			block_timestamp: Default::default(),
			tx_index_in_block: 0,
		};
		let end = CardanoPosition {
			block_hash: McBlockHash([0u8; 32]),
			block_number: end_block_no,
			block_timestamp: Default::default(),
			tx_index_in_block: u32::MAX,
		};

		let (low_bounds, high_bounds) = tokio::try_join!(
			crate::db::get_low_bounds(&pool, 0),
			crate::db::get_high_bounds(&pool, end_block_no.into()),
		)?;
		let low_bounds =
			low_bounds.ok_or("get_low_bounds(0) returned None — db-sync not initialised?")?;
		let high_bounds = high_bounds.ok_or_else(|| {
			format!("get_high_bounds({end_block_no}) returned None — block not in db-sync")
		})?;

		const LARGE_LIMIT: usize = 5_000_000;

		let paged = crate::db::PagedQuery {
			start: &start,
			end: &end,
			limit: LARGE_LIMIT,
			offset: 0,
			low_bound: low_bounds,
			high_bound: high_bounds,
		};

		let registrations: Vec<ObservedUtxo> = match auth_token_ident {
			Some(ident) => ds
				.get_registration_utxos(
					cardano_network,
					ident,
					&config.mapping_validator_address,
					&paged,
				)
				.await
				.map_err(|e| format!("get_registration_utxos: {e}"))?,
			None => vec![],
		};
		let deregistrations: Vec<ObservedUtxo> = ds
			.get_deregistration_utxos(cardano_network, &config.mapping_validator_address, &paged)
			.await
			.map_err(|e| format!("get_deregistration_utxos: {e}"))?;
		let creates: Vec<ObservedUtxo> = match cnight_ident {
			Some(ident) => ds
				.get_asset_create_utxos(cardano_network, ident, &paged)
				.await
				.map_err(|e| format!("get_asset_create_utxos: {e}"))?,
			None => vec![],
		};
		let spends: Vec<ObservedUtxo> = match cnight_ident {
			Some(ident) => ds
				.get_asset_spend_utxos(cardano_network, ident, &paged)
				.await
				.map_err(|e| format!("get_asset_spend_utxos: {e}"))?,
			None => vec![],
		};

		let mut snap = Self { registrations, deregistrations, creates, spends };
		snap.registrations.sort();
		snap.deregistrations.sort();
		snap.creates.sort();
		snap.spends.sort();
		Ok(snap)
	}
}

/// Last successful `get_utxos_up_to_capacity` call — returned directly when the
/// next call has identical `current_tip` and compatible `start_position`.
/// During initial sync many consecutive Midnight blocks share the same
/// Cardano tip, so recomputing the window each time is wasted work.
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

/// A `MidnightCNightObservationDataSource` that serves bulk observation
/// queries from an in-memory snapshot built once at startup, falling back
/// to a live db-backed source for queries past the snapshot horizon.
pub struct BulkCachedCNightObservationDataSource {
	/// All snapshot events merged into a single sorted vector. Per-call the
	/// hot path is two `partition_point`s over this vec.
	all_events: Arc<Vec<ObservedUtxo>>,
	/// Used exclusively for `get_block_by_hash` — a single indexed lookup
	/// per pallet call when the block is not yet in `block_position_cache`.
	pool: PgPool,
	/// Memoizes the current-tip hash → CardanoPosition resolution. During
	/// initial sync, dozens of Midnight blocks share the same Cardano tip,
	/// so without this every call would do a (cheap but not free) postgres
	/// round-trip.
	block_position_cache: Arc<Mutex<HashMap<McBlockHash, CardanoPosition>>>,
	/// Mirrors the DB-backed source's `LastObservation` cache.
	last_observation: Arc<Mutex<Option<LastObservation>>>,
	/// Largest Cardano block number for which the snapshot has events.
	/// Queries whose resolved tip goes past this delegate to `db_fallback`.
	snapshot_end_block: Option<u32>,
	/// Live db-backed source. Used for queries past the snapshot horizon
	/// — required because the chain advances during the node's lifetime
	/// past whatever Cardano block the bulk read picked.
	db_fallback: Arc<MidnightCNightObservationDataSourceImpl>,
	#[allow(dead_code)]
	metrics_opt: Option<MidnightDataSourceMetrics>,
}

impl BulkCachedCNightObservationDataSource {
	pub fn new(
		snapshot: CNightObservationSnapshot,
		pool: PgPool,
		db_fallback: Arc<MidnightCNightObservationDataSourceImpl>,
		metrics_opt: Option<MidnightDataSourceMetrics>,
	) -> Self {
		// Pre-merge the four sorted vecs into a single sorted vector so the
		// per-call hot path is just a pair of partition_point calls + one
		// Vec::from(slice). Each input vec is sorted ascending by
		// `tx_position`; driftsort is near-linear on the resulting 4 sorted
		// runs, and this only runs once per process.
		let CNightObservationSnapshot {
			mut registrations,
			mut deregistrations,
			mut creates,
			mut spends,
		} = snapshot;
		let total =
			registrations.len() + deregistrations.len() + creates.len() + spends.len();
		let mut all_events: Vec<ObservedUtxo> = Vec::with_capacity(total);
		all_events.append(&mut registrations);
		all_events.append(&mut deregistrations);
		all_events.append(&mut creates);
		all_events.append(&mut spends);
		all_events.sort();

		let snapshot_end_block =
			all_events.last().map(|e| e.header.tx_position.block_number);

		Self {
			all_events: Arc::new(all_events),
			pool,
			block_position_cache: Arc::new(Mutex::new(HashMap::new())),
			last_observation: Arc::new(Mutex::new(None)),
			snapshot_end_block,
			db_fallback,
			metrics_opt,
		}
	}
}

/// Helper: from a sorted vec of `ObservedUtxo`, return a slice `[a..b)` where
/// `a = first index with tx_position >= start` and
/// `b = first index with tx_position >= end` (i.e. strictly less than end).
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
		// Fast path — same-tip cache. Mirrors the DB-backed source: if
		// `current_tip` hasn't advanced since the last call, the Cardano
		// window hasn't grown, so we can reuse the previous result directly
		// (exact match) or filter it to `>= start_position` (advanced within
		// a full window — only safe when the prior call wasn't tx-capacity
		// truncated, otherwise unseen data exists past `result.end`).
		if let Ok(guard) = self.last_observation.lock() {
			if let Some(last) = guard.as_ref() {
				if last.current_tip == current_tip {
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
			}
		}

		// Resolve current_tip (a cardano block hash) to a CardanoPosition.
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

		// Hybrid delegation: if the resolved tip is past the snapshot's last
		// known event block, the query window `(snapshot_end, tip]` has events
		// we don't know about. Hand the call over to the db-backed source.
		if let Some(horizon) = self.snapshot_end_block {
			if tip_pos.block_number > horizon {
				log::debug!(
					"cNIGHT observation: tip cardano block {} past snapshot horizon {}, delegating to DB",
					tip_pos.block_number, horizon,
				);
				return self
					.db_fallback
					.get_utxos_up_to_capacity(
						config,
						start_position,
						current_tip,
						tx_capacity,
					)
					.await;
			}
		}

		let end = tip_pos.increment();

		let window = slice_range(&self.all_events, start_position, &end);

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

/// Suppress unused-type warnings from the decoded row-to-ObservedUtxo path —
/// `generate` constructs these via the existing postgres query helpers, but
/// they don't appear in this module's public surface.
#[allow(dead_code)]
fn _type_deps(
	_: CreateData,
	_: SpendData,
	_: RegistrationData,
	_: DeregistrationData,
	_: ObservedUtxoHeader,
	_: ObservedUtxoData,
	_: UtxoIndexInTx,
) {
}
