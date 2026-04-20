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

//! File-backed cNIGHT observation data source.
//!
//! For networks where the cNIGHT observation window is closed/bounded, we can
//! pre-compute every possible `get_utxos_up_to_capacity` response and ship
//! the raw events in a single file. A node loads the file at startup and
//! serves all observation queries from memory — no postgres round-trips
//! per Midnight block.
//!
//! Wire format (v1):
//!   - Scale-encoded `(Vec<ObservedUtxo>, Vec<ObservedUtxo>, Vec<ObservedUtxo>, Vec<ObservedUtxo>)`
//!     where the four vecs are `(registrations, deregistrations, creates, spends)`,
//!     each sorted ascending by `header.tx_position`.
//!
//! Future versions may add a magic-number prefix, compression (zstd), and
//! dictionary-encoded bech32 addresses.

use crate::data_source::candidates_data_source::observed_async_trait;
use crate::data_source::cnight_observation::MidnightCNightObservationDataSourceImpl;
use crate::db::MultiAssetCache;
use crate::{
	CreateData, DeregistrationData, MidnightCNightObservationDataSource, ObservedUtxo,
	ObservedUtxoData, ObservedUtxoHeader, RegistrationData, SpendData, UtxoIndexInTx,
};
use cardano_serialization_lib::{Address, EnterpriseAddress};
use midnight_primitives_cnight_observation::{CNightAddresses, CardanoPosition, ObservedUtxos};
use parity_scale_codec::{Decode, Encode};
use partner_chains_db_sync_data_sources::McFollowerMetrics;
use sha2::{Digest, Sha256};
use sidechain_domain::McBlockHash;
use sqlx::PgPool;
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

/// Wire format magic bytes + version. Bump the version if the on-disk
/// layout ever changes in a way that would confuse older nodes.
const SNAPSHOT_MAGIC: &[u8; 8] = b"MNCNIGHT";
const SNAPSHOT_VERSION: u8 = 2;

/// Inputs that uniquely identify the observation parameters the snapshot was
/// generated under. At load time the node recomputes this from its own
/// `CNightAddresses` + network magic and rejects the file if it disagrees —
/// so a preview snapshot can never be accepted on preprod, etc., even if the
/// file's content sha256 is valid.
#[derive(Clone, Debug, Encode)]
pub struct SnapshotInputs<'a> {
	pub cardano_network_magic: u32,
	pub cnight_policy_id: &'a [u8],
	pub cnight_asset_name: &'a [u8],
	pub mapping_validator_address: &'a str,
	pub auth_token_asset_name: &'a str,
}

impl SnapshotInputs<'_> {
	/// Deterministic sha256 over the scale-encoded inputs.
	pub fn hash(&self) -> [u8; 32] {
		Sha256::digest(self.encode()).into()
	}
}

/// Sorted cNIGHT observation events, grouped by kind.
#[derive(Clone, Debug, Default, Encode, Decode)]
pub struct CNightObservationSnapshot {
	pub registrations: Vec<ObservedUtxo>,
	pub deregistrations: Vec<ObservedUtxo>,
	pub creates: Vec<ObservedUtxo>,
	pub spends: Vec<ObservedUtxo>,
}

/// Loaded snapshot plus the integrity & binding hashes embedded in the file.
/// The caller should verify `inputs_hash` matches what its own config would
/// produce — see `SnapshotInputs::hash`.
pub struct LoadedSnapshot {
	pub snapshot: CNightObservationSnapshot,
	pub inputs_hash: [u8; 32],
	pub content_sha256: [u8; 32],
}

const HEADER_LEN: usize = 8 + 1 + 32 + 32; // magic + version + inputs_hash + content_sha256

impl CNightObservationSnapshot {
	/// Serialize to disk as:
	///   `magic (8) || version (1) || inputs_hash (32) || content_sha256 (32) || zstd(payload)`
	///
	/// The payload is scale-encoded then zstd-compressed (~4× smaller on
	/// mainnet). `inputs_hash` binds the snapshot to the (network-magic,
	/// policy, asset, address, auth-token) tuple it was generated under —
	/// nodes refuse the file unless their own inputs produce the same hash.
	/// `content_sha256` covers the compressed payload so integrity can be
	/// verified before any decompression work.
	pub fn write_to_path(&self, path: &Path, inputs_hash: [u8; 32]) -> std::io::Result<()> {
		let payload = self.encode();
		let compressed = zstd::bulk::compress(&payload, 19)
			.map_err(|e| std::io::Error::other(format!("zstd compress: {e}")))?;
		let content_hash: [u8; 32] = Sha256::digest(&compressed).into();
		let mut out = Vec::with_capacity(HEADER_LEN + compressed.len());
		out.extend_from_slice(SNAPSHOT_MAGIC);
		out.push(SNAPSHOT_VERSION);
		out.extend_from_slice(&inputs_hash);
		out.extend_from_slice(&content_hash);
		out.extend_from_slice(&compressed);
		std::fs::write(path, out)
	}

	/// Read from disk, verifying magic + version + content sha256. Does **not**
	/// verify `inputs_hash` — the caller must compare `LoadedSnapshot::inputs_hash`
	/// against its own `SnapshotInputs::hash` before using the snapshot.
	pub fn read_from_path(
		path: &Path,
	) -> Result<LoadedSnapshot, Box<dyn std::error::Error + Send + Sync>> {
		let bytes = std::fs::read(path)?;
		if bytes.len() < HEADER_LEN {
			return Err("snapshot file too short (missing header)".into());
		}
		if &bytes[..8] != SNAPSHOT_MAGIC {
			return Err(format!(
				"snapshot file magic mismatch: expected {SNAPSHOT_MAGIC:?}, got {:?}",
				&bytes[..8]
			)
			.into());
		}
		let version = bytes[8];
		if version != SNAPSHOT_VERSION {
			return Err(format!(
				"snapshot version mismatch: expected {SNAPSHOT_VERSION}, got {version}"
			)
			.into());
		}
		let inputs_hash: [u8; 32] = bytes[9..9 + 32].try_into().expect("slice has exact length");
		let expected_content_hash: [u8; 32] =
			bytes[9 + 32..HEADER_LEN].try_into().expect("slice has exact length");
		let compressed = &bytes[HEADER_LEN..];
		let content_sha256: [u8; 32] = Sha256::digest(compressed).into();
		if expected_content_hash != content_sha256 {
			return Err(format!(
				"snapshot content sha256 mismatch: header={}, actual={}",
				hex::encode(expected_content_hash),
				hex::encode(content_sha256),
			)
			.into());
		}
		let payload = zstd::bulk::decompress(compressed, compressed.len().saturating_mul(6))
			.map_err(|e| format!("zstd decompress: {e}"))?;
		let snapshot = Self::decode(&mut &payload[..])?;
		Ok(LoadedSnapshot { snapshot, inputs_hash, content_sha256 })
	}

	/// Total event count across all four vecs — handy for diagnostic logging.
	pub fn total_events(&self) -> usize {
		self.registrations.len()
			+ self.deregistrations.len()
			+ self.creates.len()
			+ self.spends.len()
	}

	/// Produce a snapshot by running the 4 observation queries across the
	/// whole `[0, end_block_no]` Cardano block range. Intended to be run once,
	/// offline, to generate the artifact that nodes will load at boot.
	pub async fn generate(
		pool: PgPool,
		config: &CNightAddresses,
		end_block_no: u32,
	) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
		let ds = MidnightCNightObservationDataSourceImpl::new(pool.clone(), None, 0);

		// Derive cardano_network and the mapping-validator policy ID just as
		// the live data source does.
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

		let registrations: Vec<ObservedUtxo> = match auth_token_ident {
			Some(ident) => ds
				.get_registration_utxos(
					cardano_network,
					ident,
					&config.mapping_validator_address,
					&start,
					&end,
					LARGE_LIMIT,
					0,
					low_bounds,
					high_bounds,
				)
				.await
				.map_err(|e| format!("get_registration_utxos: {e}"))?,
			None => vec![],
		};
		let deregistrations: Vec<ObservedUtxo> = ds
			.get_deregistration_utxos(
				cardano_network,
				&config.mapping_validator_address,
				&start,
				&end,
				LARGE_LIMIT,
				0,
				low_bounds,
				high_bounds,
			)
			.await
			.map_err(|e| format!("get_deregistration_utxos: {e}"))?;
		let creates: Vec<ObservedUtxo> = match cnight_ident {
			Some(ident) => ds
				.get_asset_create_utxos(
					cardano_network,
					ident,
					&start,
					&end,
					LARGE_LIMIT,
					0,
					low_bounds,
					high_bounds,
				)
				.await
				.map_err(|e| format!("get_asset_create_utxos: {e}"))?,
			None => vec![],
		};
		let spends: Vec<ObservedUtxo> = match cnight_ident {
			Some(ident) => ds
				.get_asset_spend_utxos(
					cardano_network,
					ident,
					&start,
					&end,
					LARGE_LIMIT,
					0,
					low_bounds,
					high_bounds,
				)
				.await
				.map_err(|e| format!("get_asset_spend_utxos: {e}"))?,
			None => vec![],
		};

		let mut snap = Self { registrations, deregistrations, creates, spends };
		// Each of the four vecs must be sorted ascending by position for the
		// file-backed data source's binary searches.
		snap.registrations.sort();
		snap.deregistrations.sort();
		snap.creates.sort();
		snap.spends.sort();
		Ok(snap)
	}
}

/// A `MidnightCNightObservationDataSource` that serves from an in-memory
/// snapshot, falling back to postgres only for one tiny lookup per call:
/// resolving `current_tip: McBlockHash` → block position.
pub struct FileBackedCNightObservationDataSource {
	snapshot: Arc<CNightObservationSnapshot>,
	/// Used exclusively for `get_block_by_hash` — a single indexed lookup
	/// per pallet call when the block is not yet in `block_position_cache`.
	/// All bulk observation data is served from `snapshot`.
	pool: PgPool,
	/// Memoizes the current-tip hash → CardanoPosition resolution. During
	/// initial sync, dozens of Midnight blocks share the same Cardano tip,
	/// so without this every call would do a (cheap but not free) postgres
	/// round-trip. The map grows only with distinct tips seen — bounded by
	/// the Cardano block rate over the node's lifetime.
	block_position_cache: Arc<Mutex<HashMap<McBlockHash, CardanoPosition>>>,
	/// Inputs hash embedded in the loaded snapshot file.
	snapshot_inputs_hash: [u8; 32],
	/// Network magic the pallet is running against — recomputed with the
	/// per-call `CNightAddresses` to form the expected inputs hash.
	cardano_network_magic: u32,
	/// Set to `true` after the first successful verification of the snapshot's
	/// inputs_hash against the live pallet config. Guards against paying the
	/// hash compute on every call.
	inputs_verified: Arc<std::sync::atomic::AtomicBool>,
	#[allow(dead_code)]
	metrics_opt: Option<McFollowerMetrics>,
}

impl FileBackedCNightObservationDataSource {
	pub fn new(
		snapshot: CNightObservationSnapshot,
		snapshot_inputs_hash: [u8; 32],
		cardano_network_magic: u32,
		pool: PgPool,
		metrics_opt: Option<McFollowerMetrics>,
	) -> Self {
		// NOTE: we intentionally do NOT prewarm the cache from the snapshot's
		// event `tx_position`s. `CardanoPosition` derived from `Block` uses
		// `tx_index_in_block = tx_count` (a block-end marker), whereas an
		// event's `tx_position.tx_index_in_block` is the event's tx index —
		// different semantics, using the latter as the block's end position
		// would feed the pallet wrong data and corrupt inherent verification.
		// On-demand caching still captures ~2/3 of consecutive-same-tip hits.
		Self {
			snapshot: Arc::new(snapshot),
			pool,
			block_position_cache: Arc::new(Mutex::new(HashMap::new())),
			snapshot_inputs_hash,
			cardano_network_magic,
			inputs_verified: Arc::new(std::sync::atomic::AtomicBool::new(false)),
			metrics_opt,
		}
	}

	/// Recompute the inputs hash from the pallet's live `CNightAddresses` and
	/// the network magic we were constructed with. If it doesn't match the
	/// snapshot file's embedded `inputs_hash`, the file is for a different
	/// network / policy and we must refuse to use it.
	fn verify_inputs(
		&self,
		config: &CNightAddresses,
	) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
		use std::sync::atomic::Ordering;
		if self.inputs_verified.load(Ordering::Relaxed) {
			return Ok(());
		}
		let expected = SnapshotInputs {
			cardano_network_magic: self.cardano_network_magic,
			cnight_policy_id: &config.cnight_policy_id,
			cnight_asset_name: config.cnight_asset_name.as_bytes(),
			mapping_validator_address: &config.mapping_validator_address,
			auth_token_asset_name: &config.auth_token_asset_name,
		}
		.hash();
		if expected != self.snapshot_inputs_hash {
			return Err(format!(
				"cNIGHT observation snapshot inputs_hash mismatch: \
				 file={} (built for different network / policy / addresses), \
				 pallet expects={}",
				hex::encode(self.snapshot_inputs_hash),
				hex::encode(expected),
			)
			.into());
		}
		self.inputs_verified.store(true, Ordering::Relaxed);
		log::info!(
			"cNIGHT observation snapshot verified (inputs_hash={}, network_magic={})",
			hex::encode(self.snapshot_inputs_hash),
			self.cardano_network_magic,
		);
		Ok(())
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
impl MidnightCNightObservationDataSource for FileBackedCNightObservationDataSource {
	async fn get_utxos_up_to_capacity(
		&self,
		config: &CNightAddresses,
		start_position: &CardanoPosition,
		current_tip: McBlockHash,
		tx_capacity: usize,
	) -> Result<ObservedUtxos, Box<dyn std::error::Error + Send + Sync>> {
		self.verify_inputs(config)?;
		// Resolve current_tip (a cardano block hash) to a CardanoPosition.
		// Fast path: memoized from a prior call or the snapshot prewarm.
		let cached = self
			.block_position_cache
			.lock()
			.ok()
			.and_then(|g| g.get(&current_tip).cloned());
		let end: CardanoPosition = match cached {
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
		let end = end.increment();

		let regs = slice_range(&self.snapshot.registrations, start_position, &end);
		let deregs = slice_range(&self.snapshot.deregistrations, start_position, &end);
		let creates = slice_range(&self.snapshot.creates, start_position, &end);
		let spends = slice_range(&self.snapshot.spends, start_position, &end);

		let total = regs.len() + deregs.len() + creates.len() + spends.len();
		let mut merged: Vec<ObservedUtxo> = Vec::with_capacity(total);
		merged.extend(regs.iter().cloned());
		merged.extend(deregs.iter().cloned());
		merged.extend(creates.iter().cloned());
		merged.extend(spends.iter().cloned());
		merged.sort();

		// Truncate to `tx_capacity` whole transactions.
		let mut truncated: Vec<ObservedUtxo> = Vec::with_capacity(merged.len());
		let mut num_txs: usize = 0;
		let mut cur_tx: Option<CardanoPosition> = None;
		for utxo in merged {
			if cur_tx.as_ref().is_none_or(|tx| tx < &utxo.header.tx_position) {
				num_txs += 1;
				cur_tx = Some(utxo.header.tx_position.clone());
			}
			if num_txs == tx_capacity {
				break;
			}
			truncated.push(utxo);
		}

		let result_end = if num_txs < tx_capacity {
			end
		} else {
			truncated
				.last()
				.map(|u| u.header.tx_position.clone())
				.unwrap_or_else(|| start_position.clone())
				.increment()
		};

		Ok(ObservedUtxos {
			start: start_position.clone(),
			end: result_end,
			utxos: truncated,
		})
	}
}
);

/// Suppress unused-type warnings from the decoded row-to-ObservedUtxo path —
/// the generator uses these via the existing postgres query helpers.
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
