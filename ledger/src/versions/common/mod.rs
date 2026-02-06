// This file is part of midnight-node.
// Copyright (C) 2025 Midnight Foundation
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

#[cfg(feature = "std")]
use super::{
	base_crypto_local, coin_structure_local, helpers_local, ledger_storage_local,
	midnight_serialize_local, mn_ledger_local, onchain_runtime_local, transient_crypto_local,
	zswap_local,
};

#[cfg(feature = "std")]
use midnight_serialize_local::Tagged;
#[cfg(feature = "std")]
use sha2::digest::{OutputSizeUser, generic_array::typenum::U32};
#[cfg(feature = "std")]
use transient_crypto_local::commitment::PureGeneratorPedersen;

use alloc::vec::Vec;
use sp_externalities::{Externalities, ExternalitiesExt};

pub mod types;
use types::LedgerApiError;

#[cfg(feature = "std")]
pub mod api;

#[cfg(feature = "std")]
pub mod conversions;

#[cfg(feature = "std")]
use {
	api::{
		ContractAddress, ContractState, Ledger, LedgerParameters, SystemTransaction, Transaction,
		TransactionAppliedStage, TransactionOperation,
	},
	base_crypto_local::{
		cost_model::NormalizedCost as LedgerNormalizedCost, hash::HashOutput, time::Timestamp,
	},
	coin_structure_local::coin::Nonce,
	ledger_storage_local::{
		Storage,
		arena::{ArenaKey, Sp, TypedArenaKey},
		db::{DB, ParityDb},
		storage::{default_storage, set_default_storage},
	},
	midnight_primitives_ledger::{LedgerMetricsExt, LedgerStorageExt},
	mn_ledger_local::{
		dust::InitialNonce,
		structure::{
			CNightGeneratesDustActionType, CNightGeneratesDustEvent, ClaimKind, ContractAction,
			MaintenanceUpdate, ProofMarker, SignatureKind, SingleUpdate,
			Transaction as LedgerTransaction, VerifiedTransaction,
		},
	},
	std::time::Instant,
};

use crate::common::types::{
	BlockContext, ContractCallsDetails, FallibleCoinsDetails, GasCost, GuaranteedCoinsDetails,
	Hash, Op, SystemTransactionAppliedStateRoot, TransactionAppliedStateRoot, TransactionDetails,
	Tx,
};

#[cfg(feature = "std")]
use {lazy_static::lazy_static, moka::sync::Cache};

pub const LOG_TARGET: &str = "midnight::ledger_v2";
pub const MINT_COINS_DOMAIN_SEPARATOR: &[u8; 10] = b"mint_coins";

#[derive(PartialEq, Eq, Hash)]
pub struct NextStateRootKey {
	state_hash: Hash,
	tx_hash: Hash,
	runtime_version: u32,
}

#[cfg(feature = "std")]
#[derive(Clone)]
pub struct CachedNextState {
	state_root: Vec<u8>,
	failed_segment_indices: Vec<u16>,
}

#[derive(PartialEq, Eq, Hash)]
pub struct MempoolValidationCacheKey {
	tx_hash: Hash,
	runtime_version: u32,
}

#[cfg(feature = "std")]
lazy_static! {
	/// Next-state cache: stores the serialized next ledger state root and failed segment indices.
	///
	/// Populated by `validate_guaranteed_execution` (pre-dispatch) so that `apply_transaction`
	/// can skip recomputing the expensive ZK verification + state transition.
	static ref NEXT_STATE_ROOT_CACHE: Cache<NextStateRootKey, CachedNextState> =
		Cache::new(1000);

	/// Soft cache: stores validation result for mempool revalidation.
	/// No type erasure needed since Result<(), LedgerApiError> is not generic.
	static ref MEMPOOL_VALIDATION_CACHE: Cache<MempoolValidationCacheKey, Result<(), LedgerApiError>> =
		Cache::new(1000);
}

#[cfg(feature = "std")]
pub struct Bridge<S: SignatureKind<D>, D: DB> {
	_phantom: core::marker::PhantomData<(S, D)>,
}

#[cfg(feature = "std")]
impl<S: SignatureKind<D> + std::fmt::Debug, D: DB> Bridge<S, D>
where
	mn_ledger_local::structure::Transaction<S, ProofMarker, PureGeneratorPedersen, D>: Tagged,
	D::Hasher: OutputSizeUser<OutputSize = U32>,
{
	pub fn set_default_storage(mut externalities: &mut dyn Externalities) {
		let maybe_storage = externalities.extension::<LedgerStorageExt>();
		if let Some(storage) = maybe_storage {
			let res = set_default_storage(|| {
				let db = ParityDb::<sha2::Sha256>::open(storage.0.db_path.as_path());
				Storage::new(storage.0.cache_size, db)
			});
			if res.is_err() {
				log::warn!("Warning: Failed to set default storage: {res:?}");
			}
		} else {
			log::error!(
				target: LOG_TARGET,
				"Ledger Storage Externality should be always present!!",
			);
		}
	}

	pub fn pre_fetch_storage(
		mut externalities: &mut dyn Externalities,
		state_key: &[u8],
	) -> Result<(), LedgerApiError> {
		let api = api::new();
		let typed_key: TypedArenaKey<Ledger<D>, D::Hasher> = api.tagged_deserialize(state_key)?;
		let key: ArenaKey<D::Hasher> = typed_key.into();

		let now = std::time::Instant::now();
		default_storage::<D>().with_backend(|backend| backend.pre_fetch(key.hash(), None, true));
		let elapsed = now.elapsed().as_secs_f64();

		let maybe_metrics = externalities.extension::<LedgerMetricsExt>();
		if let Some(metrics) = maybe_metrics {
			metrics.observe_storage_fetch_time(elapsed, "ledger_state");
		}
		Ok(())
	}

	pub fn flush_storage(mut externalities: &mut dyn Externalities) {
		let now = std::time::Instant::now();
		default_storage::<D>().with_backend(|backend| backend.flush_all_changes_to_db());
		let elapsed = now.elapsed().as_secs_f64();

		let maybe_metrics = externalities.extension::<LedgerMetricsExt>();
		if let Some(metrics) = maybe_metrics {
			metrics.observe_storage_flush_time(elapsed, "ledger_state");
		}
	}

	pub fn post_block_update(
		mut _externalities: &mut dyn Externalities,
		state_key: &[u8],
		block_context: BlockContext,
	) -> Result<Vec<u8>, LedgerApiError> {
		let api = api::new();
		let ledger = Self::get_ledger(&api, state_key)?;

		let mut ledger = Ledger::post_block_update(ledger, block_context).map_err(|e| {
			log::error!(
				target: LOG_TARGET,
				"Post Block Update error: {e:?}"
			);
			LedgerApiError::NoLedgerState
		})?;

		let state_root = api.tagged_serialize(&ledger.as_typed_key())?;

		// Only update state after no errors
		ledger.persist();

		Ok(state_root)
	}

	pub fn get_version() -> Vec<u8> {
		crate::utils::find_crate_version(super::CRATE_NAME).unwrap_or(b"unknown".into())
	}

	pub fn apply_transaction(
		mut externalities: &mut dyn Externalities,
		state_key: &[u8],
		tx_serialized: &[u8],
		block_context: BlockContext,
		should_skip_failed_segments: bool,
		runtime_version: u32,
	) -> Result<TransactionAppliedStateRoot, LedgerApiError>
	where
		VerifiedTransaction<D>: Send + Sync + 'static,
	{
		// Gather metrics for Prometheus
		let start_tx_processing_time = Instant::now();
		let tx_size = tx_serialized.len();

		let api = api::new();
		let tx = api.tagged_deserialize::<Transaction<S, D>>(tx_serialized)?;
		let tx_hash = tx.hash();
		log::info!(
			target: LOG_TARGET,
			"📥 Applying transaction {}",
			hex::encode(tx_hash)
		);
		let ledger = Self::get_ledger(&api, state_key)?;
		let initial_utxos_size = ledger.state.utxo.utxos.size();

		let (mut new_ledger, failed_segment_indices, _was_cached) =
			Self::get_next_ledger_state(&api, ledger, &tx, &block_context, runtime_version)?;

		let all_applied = failed_segment_indices.is_empty();
		let mut utxos = tx.unshielded_utxos();
		utxos.remove_failed_segments(&failed_segment_indices);

		let failed_segments = if !all_applied { Some(failed_segment_indices) } else { None };

		let operations =
			tx.calls_and_deploys(should_skip_failed_segments.then_some(failed_segments).flatten());

		let (utxo_outputs, utxo_inputs) =
			utxos.check_utxos_response_integrity(initial_utxos_size, &new_ledger)?;

		let mut event = TransactionAppliedStateRoot {
			state_root: api.tagged_serialize(&new_ledger.as_typed_key())?,
			tx_hash,
			all_applied,
			call_addresses: vec![],
			deploy_addresses: vec![],
			maintain_addresses: vec![],
			claim_rewards: vec![],
			unshielded_utxos_created: utxo_outputs,
			unshielded_utxos_spent: utxo_inputs,
		};

		for op in operations {
			match op {
				TransactionOperation::Call { address, .. } => {
					event.call_addresses.push(api.tagged_serialize(&address)?);
				},
				TransactionOperation::Deploy { address } => {
					event.deploy_addresses.push(api.tagged_serialize(&address)?);
				},
				TransactionOperation::Maintain { address } => {
					event.maintain_addresses.push(api.tagged_serialize(&address)?);
				},
				TransactionOperation::ClaimRewards { value, .. } => {
					event.claim_rewards.push(value);
				},
			}
		}

		// Only update state after no errors
		new_ledger.persist();

		// Write Prometheus metrics
		let maybe_metrics = externalities.extension::<LedgerMetricsExt>();
		if let Some(metrics) = maybe_metrics {
			let tx_type = Self::get_tx_type(&tx);
			let elapsed_time = start_tx_processing_time.elapsed().as_secs_f64();

			metrics.observe_txs_processing_time(elapsed_time, tx_type);
			metrics.observe_txs_size(tx_size as f64, tx_type);
		}

		Ok(event)
	}

	pub fn apply_system_transaction(
		mut externalities: &mut dyn Externalities,
		state_key: &[u8],
		tx_serialized: &[u8],
		block_context: BlockContext,
	) -> Result<SystemTransactionAppliedStateRoot, LedgerApiError> {
		// Gather metrics for Prometheus
		let start_system_tx_processing_time = Instant::now();
		let tx_size = tx_serialized.len();

		let api = api::new();
		let tx = api.tagged_deserialize::<SystemTransaction>(tx_serialized)?;
		let tx_type = Self::get_system_tx_type(&tx);
		log::info!(
			target: LOG_TARGET,
			"⚙️  Processing SystemTx {tx:?}"
		);
		let tx_hash = tx.transaction_hash().0.0;
		let ledger = Self::get_ledger(&api, state_key)?;

		let mut ledger =
			Ledger::apply_system_tx(ledger, &tx, Timestamp::from_secs(block_context.tblock))?;

		let event = SystemTransactionAppliedStateRoot {
			state_root: api.tagged_serialize(&ledger.as_typed_key())?,
			tx_hash,
			tx_type: tx_type.to_string(),
		};

		// Only update state after no errors
		ledger.persist();

		// Write Prometheus metrics
		let maybe_metrics = externalities.extension::<LedgerMetricsExt>();
		if let Some(metrics) = maybe_metrics {
			let elapsed_time = start_system_tx_processing_time.elapsed().as_secs_f64();

			metrics.observe_system_txs_processing_time(elapsed_time, tx_type);
			metrics.observe_txs_size(tx_size as f64, tx_type);
		}

		Ok(event)
	}

	pub fn validate_transaction(
		mut externalities: &mut dyn Externalities,
		state_key: &[u8],
		tx_serialized: &[u8],
		block_context: BlockContext,
		runtime_version: u32,
		// The runtime's max weight as of now
		max_weight: u64,
		get_tx_details: bool,
	) -> Result<(Hash, Option<TransactionDetails>), LedgerApiError> {
		// Gather metrics for Prometheus
		let start_tx_validation_time = Instant::now();

		let api = api::new();
		let tx = api.tagged_deserialize::<Transaction<S, D>>(tx_serialized)?;
		let ledger = Self::get_ledger(&api, state_key)?;

		let tx_hash = tx.hash();

		let was_cached = Self::do_validate_transaction(
			&api,
			ledger.clone(),
			&tx,
			&block_context,
			runtime_version,
		)?;

		let tx_details = if get_tx_details {
			let tx_gas_cost =
				Self::get_transaction_cost(state_key, tx_serialized, &block_context, max_weight)?;

			Some(Self::get_transaction_details(&tx, &ledger, tx_gas_cost)?)
		} else {
			None
		};

		// Write Prometheus metrics
		if let Some(metrics) = externalities.extension::<LedgerMetricsExt>() {
			// Record cache hit/miss metrics
			if was_cached {
				metrics.inc_tx_validation_cache_hit("mempool");
			} else {
				metrics.inc_tx_validation_cache_miss();
				// Only record validation time on cache miss (when actual work was done)
				let tx_type = Self::get_tx_type(&tx);
				let elapsed_time = start_tx_validation_time.elapsed().as_secs_f64();
				metrics.observe_txs_validating_time(elapsed_time, tx_type);
			}

			// Report current cache sizes
			metrics.set_tx_validation_cache_size(
				"next_state_root",
				NEXT_STATE_ROOT_CACHE.entry_count(),
			);
			metrics.set_tx_validation_cache_size("mempool", MEMPOOL_VALIDATION_CACHE.entry_count());
		}

		Ok((tx_hash, tx_details))
	}

	/// Validates that applying a transaction will succeed.
	///
	/// Used by `pre_dispatch` to reject transactions whose application
	/// would fail - this keeps the block free of failed transactions.
	///
	/// Computes the full next ledger state and caches it in `NEXT_STATE_ROOT_CACHE`
	/// so that `apply_transaction` can skip recomputation.
	pub fn validate_guaranteed_execution(
		mut externalities: &mut dyn Externalities,
		state_key: &[u8],
		tx_serialized: &[u8],
		block_context: BlockContext,
		runtime_version: u32,
	) -> Result<(), LedgerApiError>
	where
		VerifiedTransaction<D>: Send + Sync + 'static,
	{
		let api = api::new();
		let tx = api.tagged_deserialize::<Transaction<S, D>>(tx_serialized)?;
		let ledger = Self::get_ledger(&api, state_key)?;

		// Perform full validation and cache the next state
		let was_cached = Self::do_validate_guaranteed_execution(
			&api,
			ledger,
			&tx,
			&block_context,
			runtime_version,
		)?;

		// Write Prometheus metrics
		if let Some(metrics) = externalities.extension::<LedgerMetricsExt>() {
			if was_cached {
				metrics.inc_tx_validation_cache_hit("next_state_root");
			} else {
				metrics.inc_tx_validation_cache_miss();
			}

			// Report current cache sizes
			metrics.set_tx_validation_cache_size(
				"next_state_root",
				NEXT_STATE_ROOT_CACHE.entry_count(),
			);
			metrics.set_tx_validation_cache_size("mempool", MEMPOOL_VALIDATION_CACHE.entry_count());
		}

		Ok(())
	}

	pub fn get_decoded_transaction(transaction_bytes: &[u8]) -> Result<Tx, LedgerApiError> {
		let api = api::new();
		let tx = api.tagged_deserialize::<Transaction<S, D>>(transaction_bytes)?;
		let hash = tx.hash();
		let operations = tx.calls_and_deploys(None).try_fold(Vec::new(), |mut acc, cd| {
			let a = match cd {
				TransactionOperation::Call { address, entry_point } => {
					Op::Call { address: api.tagged_serialize(&address)?, entry_point }
				},
				TransactionOperation::Deploy { address } => {
					Op::Deploy { address: api.tagged_serialize(&address)? }
				},
				TransactionOperation::Maintain { address } => {
					Op::Maintain { address: api.tagged_serialize(&address)? }
				},
				TransactionOperation::ClaimRewards { value } => Op::ClaimRewards { value },
			};
			acc.push(a);
			Ok::<_, LedgerApiError>(acc)
		})?;

		let identifiers = tx.identifiers().try_fold(Vec::new(), |mut acc, i| {
			acc.push(api.tagged_serialize(&i)?);
			Ok::<_, LedgerApiError>(acc)
		})?;

		Ok(Tx {
			hash,
			operations,
			identifiers,
			has_fallible_coins: tx.has_fallible_coins(),
			has_guaranteed_coins: tx.has_guaranteed_coins(),
		})
	}

	fn do_get_contract_state<F>(
		api: &api::Api,
		state_key: &[u8],
		contract_address: &[u8],
		f: F,
	) -> Result<Vec<u8>, LedgerApiError>
	where
		F: FnOnce(ContractState<D>) -> Result<Vec<u8>, LedgerApiError>,
	{
		let addr = api.deserialize::<ContractAddress>(contract_address)?;
		let ledger = Self::get_ledger(api, state_key)?;

		ledger.get_contract_state(addr).map_or(Ok(Vec::new()), f)
	}

	pub fn get_contract_state(
		state_key: &[u8],
		contract_address: &[u8],
	) -> Result<Vec<u8>, LedgerApiError> {
		let api = api::new();

		let f = |contract_state| api.tagged_serialize(&contract_state);

		Self::do_get_contract_state(&api, state_key, contract_address, f)
	}

	pub fn get_zswap_chain_state(
		state_key: &[u8],
		contract_address: &[u8],
	) -> Result<Vec<u8>, LedgerApiError> {
		let api = api::new();
		let addr = api.deserialize::<ContractAddress>(contract_address)?;
		let ledger = Self::get_ledger(&api, state_key)?;

		api.tagged_serialize(&ledger.get_zswap_state(Some(addr)))
	}

	pub fn get_zswap_state_root(state_key: &[u8]) -> Result<Vec<u8>, LedgerApiError> {
		let api = api::new();
		let ledger = Self::get_ledger(&api, state_key)?;

		api.serialize(&ledger.get_zswap_state_root())
	}

	pub fn get_unclaimed_amount(
		state_key: &[u8],
		beneficiary: &[u8],
	) -> Result<u128, LedgerApiError> {
		let api = api::new();

		let night_addr = api.night_address(beneficiary)?;
		let ledger = Self::get_ledger(&api, state_key)?;

		Ok(*ledger.get_unclaimed_amount(night_addr).unwrap_or(&0))
	}

	pub fn get_ledger_parameters(state_key: &[u8]) -> Result<Vec<u8>, LedgerApiError> {
		let api = api::new();
		let ledger = Self::get_ledger(&api, state_key)?;
		let ledger_parameters = Self::get_deserialized_ledger_parameters(&ledger);
		api.tagged_serialize(&ledger_parameters)
	}

	// TODO COST MODEL: Needs to be redone with the new ledger cost model
	#[allow(unused_variables)]
	pub fn get_transaction_cost(
		state_key: &[u8],
		tx: &[u8],
		block_context: &BlockContext,
		max_weight: u64,
	) -> Result<GasCost, LedgerApiError> {
		let api = api::new();
		let tx = api.tagged_deserialize::<Transaction<S, D>>(tx)?;
		let ledger = Self::get_ledger(&api, state_key)?;

		let cost =
			tx.0.cost(&ledger.state.parameters, true)
				.map_err(|_| LedgerApiError::FeeCalculationError)?;

		let limits = ledger.state.parameters.limits.block_limits;
		let normalized = cost.normalize(limits).ok_or(LedgerApiError::BlockLimitExceededError)?;

		let gas_cost = scale_normalized_cost(&normalized, max_weight);

		Ok(gas_cost)
	}

	fn get_deserialized_ledger_parameters(state: &Ledger<D>) -> LedgerParameters {
		state.get_parameters()
	}

	fn get_ledger(api: &api::Api, state_key: &[u8]) -> Result<Sp<Ledger<D>, D>, LedgerApiError> {
		let key: TypedArenaKey<Ledger<D>, D::Hasher> = api.tagged_deserialize(state_key)?;
		default_storage().arena.get_lazy(&key).map_err(|e| {
			log::error!(target: LOG_TARGET, "Error loading Ledger State: {e:?}");
			LedgerApiError::NoLedgerState
		})
	}

	fn get_transaction_details(
		tx: &Transaction<S, D>,
		_ledger: &Ledger<D>,
		tx_gas_cost: GasCost,
	) -> Result<TransactionDetails, LedgerApiError> {
		let ledger_tx = &tx.0;

		match ledger_tx {
			LedgerTransaction::Standard(tx) => {
				let guaranteed_coins = GuaranteedCoinsDetails::new(
					tx.guaranteed_inputs().count() as u32,
					tx.guaranteed_outputs().count() as u32,
					tx.guaranteed_transients().count() as u32,
				);

				let fallible_coins_details = FallibleCoinsDetails::new(
					tx.fallible_inputs().count() as u32,
					tx.fallible_outputs().count() as u32,
					tx.fallible_transients().count() as u32,
				);

				let mut contract_calls = tx.actions().try_fold(
					ContractCallsDetails::default(),
					|mut cd, (_segment, action)| {
						match action {
							ContractAction::Call(_) => {
								cd.inc_calls();
							},
							ContractAction::Deploy(_) => {
								cd.inc_deploys();
							},
							ContractAction::Maintain(MaintenanceUpdate { updates, .. }) => {
								for update in updates.iter() {
									match *update {
										SingleUpdate::ReplaceAuthority(..) => {
											cd.inc_replace_authority();
										},
										SingleUpdate::VerifierKeyInsert(..) => {
											cd.inc_verifier_key_insert();
										},
										SingleUpdate::VerifierKeyRemove(..) => {
											cd.inc_verifier_key_remove();
										},
									}
								}
							},
						};
						Ok(cd)
					},
				)?;

				contract_calls.set_gas_cost(tx_gas_cost);

				Ok(TransactionDetails::Standard {
					guaranteed_coins,
					fallible_coins: fallible_coins_details,
					contract_calls,
				})
			},
			LedgerTransaction::ClaimRewards(_) => Ok(TransactionDetails::ClaimRewards),
		}
	}

	fn get_tx_type(tx: &Transaction<S, D>) -> &'static str {
		match tx.0 {
			mn_ledger_local::structure::Transaction::Standard(_) => "standard",
			mn_ledger_local::structure::Transaction::ClaimRewards(_) => "claim_rewards",
		}
	}

	fn get_system_tx_type(tx: &SystemTransaction) -> &'static str {
		match tx {
			SystemTransaction::OverwriteParameters(_) => "overwrite_parameters",
			SystemTransaction::DistributeNight(claim_kind, _) => match claim_kind {
				ClaimKind::Reward => "distribute_night_reward",
				ClaimKind::CardanoBridge => "distribute_night_cardano_bridge",
			},
			SystemTransaction::PayBlockRewardsToTreasury { .. } => "pay_block_rewards_to_treasury",
			SystemTransaction::PayFromTreasuryShielded { .. } => "pay_from_treasury_shielded",
			SystemTransaction::PayFromTreasuryUnshielded { .. } => "pay_from_treasury_unshielded",
			SystemTransaction::DistributeReserve(_) => "distribute_reserve",
			SystemTransaction::CNightGeneratesDustUpdate { .. } => "cnight_generates_dust_update",
			_ => "unknown",
		}
	}

	/// Computes a VerifiedTransaction by running ZK proof verification.
	fn get_verified_transaction(
		ledger: &Ledger<D>,
		tx: &Transaction<S, D>,
		block_context: &BlockContext,
	) -> Result<VerifiedTransaction<D>, LedgerApiError> {
		let ctx = ledger.get_transaction_context(block_context.clone());
		tx.0.well_formed(
			&ctx.ref_state,
			mn_ledger_local::verify::WellFormedStrictness::default(),
			ctx.block_context.tblock,
		)
		.map_err(|e| LedgerApiError::Transaction(types::TransactionError::Malformed(e.into())))
	}

	/// Computes the next ledger state, using the `NEXT_STATE_ROOT_CACHE` when possible.
	///
	/// On cache hit: deserializes the cached state root and loads the ledger from the arena.
	/// On cache miss: verifies the transaction and applies it, then caches the result.
	///
	/// Returns `(new_ledger, failed_segment_indices, was_cached)`.
	/// `all_applied` can be derived via `failed_segment_indices.is_empty()`.
	#[allow(clippy::type_complexity)]
	fn get_next_ledger_state(
		api: &api::Api,
		ledger: Sp<Ledger<D>, D>,
		tx: &Transaction<S, D>,
		block_context: &BlockContext,
		runtime_version: u32,
	) -> Result<(Sp<Ledger<D>, D>, Vec<u16>, bool), LedgerApiError> {
		let cache_key = NextStateRootKey {
			state_hash: ledger.state.state_hash().0.into(),
			tx_hash: tx.hash(),
			runtime_version,
		};

		// Check cache
		if let Some(cached) = NEXT_STATE_ROOT_CACHE.get(&cache_key) {
			let typed_key: TypedArenaKey<Ledger<D>, D::Hasher> =
				api.tagged_deserialize(&cached.state_root)?;
			let new_ledger = default_storage().arena.get_lazy(&typed_key).map_err(|e| {
				log::error!(target: LOG_TARGET, "Error loading cached next state: {e:?}");
				LedgerApiError::NoLedgerState
			})?;
			return Ok((new_ledger, cached.failed_segment_indices.clone(), true));
		}

		// Cache miss: compute from scratch
		let verified_tx = Self::get_verified_transaction(&ledger, tx, block_context)?;
		let tx_ctx = ledger.get_transaction_context(block_context.clone());
		let (new_ledger, applied_stage) =
			Ledger::apply_verified_transaction(ledger, api, tx, &verified_tx, &tx_ctx)?;

		let failed_segment_indices = match &applied_stage {
			TransactionAppliedStage::AllApplied => vec![],
			TransactionAppliedStage::PartialSuccess(segments) => segments
				.iter()
				.filter(|(_, result)| result.is_err())
				.map(|(id, _)| *id)
				.collect(),
		};

		// Cache the result
		let state_root = api.tagged_serialize(&new_ledger.as_typed_key())?;
		NEXT_STATE_ROOT_CACHE.insert(
			cache_key,
			CachedNextState { state_root, failed_segment_indices: failed_segment_indices.clone() },
		);

		Ok((new_ledger, failed_segment_indices, false))
	}

	/// Validates a transaction for the mempool using the soft cache.
	///
	/// Uses `tx_hash` for quick revalidation of transactions already in the pool.
	/// The soft cache prevents redundant ZK proof verification for mempool housekeeping.
	///
	/// Returns `true` if the validation was served from cache, `false` if validation was performed.
	fn do_validate_transaction(
		api: &api::Api,
		ledger: Sp<Ledger<D>, D>,
		tx: &Transaction<S, D>,
		block_context: &BlockContext,
		runtime_version: u32,
	) -> Result<bool, LedgerApiError>
	where
		VerifiedTransaction<D>: Send + Sync + 'static,
	{
		let cache_key = MempoolValidationCacheKey { tx_hash: tx.hash(), runtime_version };

		// Check soft cache first (quick tx_hash-only lookup for mempool revalidation)
		if let Some(cached) = MEMPOOL_VALIDATION_CACHE.get(&cache_key) {
			return cached.map(|_| true);
		}

		// Cache miss: compute next state (also populates NEXT_STATE_ROOT_CACHE)
		let tx_hash_hex = hex::encode(cache_key.tx_hash);
		match Self::get_next_ledger_state(api, ledger, tx, block_context, runtime_version) {
			Ok(_) => {
				log::info!(
					target: LOG_TARGET,
					"📋 Validated transaction {} for mempool",
					tx_hash_hex
				);
				MEMPOOL_VALIDATION_CACHE.insert(cache_key, Ok(()));
				Ok(false)
			},
			Err(e) => {
				log::warn!(
					target: LOG_TARGET,
					"🚫 Rejected transaction {} from mempool: {e}",
					tx_hash_hex
				);
				Err(e)
			},
		}
	}

	/// Validates transaction application by computing and caching the next state.
	///
	/// Calls `get_next_ledger_state` which computes the full next state and
	/// caches it in `NEXT_STATE_ROOT_CACHE` for later use by `apply_transaction`.
	///
	/// Returns `true` if validation was served from the cache, `false` otherwise.
	fn do_validate_guaranteed_execution(
		api: &api::Api,
		ledger: Sp<Ledger<D>, D>,
		tx: &Transaction<S, D>,
		block_context: &BlockContext,
		runtime_version: u32,
	) -> Result<bool, LedgerApiError>
	where
		VerifiedTransaction<D>: Send + Sync + 'static,
	{
		let tx_hash = tx.hash();

		// Invalidate mempool cache — tx must re-validate after a block authoring attempt
		MEMPOOL_VALIDATION_CACHE
			.invalidate(&MempoolValidationCacheKey { tx_hash, runtime_version });

		let (_new_ledger, _failed_segment_indices, was_cached) =
			Self::get_next_ledger_state(api, ledger, tx, block_context, runtime_version)?;

		Ok(was_cached)
	}

	pub fn construct_cnight_generates_dust_event(
		value: u128,
		owner: &[u8],
		time: u64,
		action: u8,
		nonce: [u8; 32],
	) -> Result<Vec<u8>, LedgerApiError> {
		let api = api::new();
		let event = CNightGeneratesDustEvent {
			value,
			owner: api.deserialize(owner)?,
			time: Timestamp::from_secs(time),
			action: match action {
				0 => Ok(CNightGeneratesDustActionType::Create),
				1 => Ok(CNightGeneratesDustActionType::Destroy),
				_ => Err(LedgerApiError::Deserialization(
					api::DeserializationError::CNightGeneratesDustActionType,
				)),
			}?,
			nonce: InitialNonce(HashOutput(nonce)),
		};
		api.tagged_serialize(&event)
	}

	pub fn construct_cnight_generates_dust_system_tx(
		events: Vec<Vec<u8>>,
	) -> Result<Vec<u8>, LedgerApiError> {
		let api = api::new();
		let events: Result<Vec<CNightGeneratesDustEvent>, LedgerApiError> =
			events.iter().map(|e| api.tagged_deserialize(e)).collect();
		let system_tx = SystemTransaction::CNightGeneratesDustUpdate { events: events? };
		api.tagged_serialize(&system_tx)
	}
}

/// Creates a Nonce using BlakeTwo256; similar Hashing type set in the Runtime.
///
/// # Arguments
/// * `separator` - an indicator from which this nonce belongs to.
/// * `block_hash`
/// * `output_number` - its position in the list
#[cfg(feature = "std")]
#[allow(dead_code)]
fn create_nonce(separator: &[u8], block_hash: &[u8], output_number: u8) -> Nonce {
	use sp_runtime::traits::{BlakeTwo256, Hash};

	let concatenated = [block_hash, separator, &[output_number]].concat();

	let h256 = BlakeTwo256::hash(&concatenated);

	Nonce(HashOutput(h256.0))
}

#[cfg(feature = "std")]
fn scale_normalized_cost(normalized: &LedgerNormalizedCost, max_weight: u64) -> GasCost {
	let max_fp = *[
		normalized.read_time,
		normalized.compute_time,
		normalized.block_usage,
		normalized.bytes_written,
		normalized.bytes_churned,
	]
	.iter()
	.max()
	.expect("Hard-coded array should not be empty");

	max_fp.into_atomic_units(max_weight as u128).min(max_weight as u128) as u64
}

#[cfg(test)]
mod tests {
	use super::*;
	use base_crypto_local::cost_model::FixedPoint;

	fn normalized_all(value: FixedPoint) -> LedgerNormalizedCost {
		LedgerNormalizedCost {
			read_time: value,
			compute_time: value,
			block_usage: value,
			bytes_written: value,
			bytes_churned: value,
		}
	}

	#[test]
	fn scale_normalized_cost_bounds_and_monotonic() {
		let max_weight = 100u64;

		let zero = scale_normalized_cost(&normalized_all(FixedPoint::from(0.0f64)), max_weight);
		let half = scale_normalized_cost(&normalized_all(FixedPoint::from(0.5f64)), max_weight);
		let one = scale_normalized_cost(&normalized_all(FixedPoint::from(1.0f64)), max_weight);
		let over_one = scale_normalized_cost(&normalized_all(FixedPoint::from(1.5f64)), max_weight);
		let negative =
			scale_normalized_cost(&normalized_all(FixedPoint::from(-0.25f64)), max_weight);

		assert_eq!(zero, 0);
		assert_eq!(negative, 0);
		assert!(half >= max_weight / 2 && half <= max_weight);
		assert_eq!(one, max_weight);
		assert_eq!(over_one, max_weight);
		assert!(half >= zero);
		assert!(one >= half);
	}
}
