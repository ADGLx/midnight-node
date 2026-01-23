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

use super::{
	ArenaKey, BlockContext, DB, DUST_EXPECTED_FILES, DustResolver, Event, FetchMode, LedgerState,
	Loader, MidnightDataProvider, NormalizedCost, Offer, OutputMode, PUBLIC_PARAMS, ProofKind,
	PureGeneratorPedersen, Resolver, SerdeTransaction, SignatureKind, Storable, SyntheticCost,
	Tagged, Timestamp, Transaction, TransactionContext, TransactionResult, Utxo,
	VerifiedTransaction, Wallet, WalletAddress, WalletSeed, WellFormedStrictness,
	compute_overall_fullness, default_storage, mn_ledger_serialize as serialize,
	mn_ledger_storage as storage, types::StorableSyntheticCost,
};
use derive_where::derive_where;
use hex::{ToHex, encode as hex_encode};
use lazy_static::lazy_static;
use std::{
	collections::{HashMap, HashSet},
	fs::OpenOptions,
	io::Write,
	sync::Mutex,
	time::{SystemTime, UNIX_EPOCH},
};
use tokio::sync::Mutex as MutexTokio;

lazy_static! {
	pub static ref DEFAULT_RESOLVER: Resolver = Resolver::new(
		PUBLIC_PARAMS.clone(),
		DustResolver(
			MidnightDataProvider::new(
				FetchMode::OnDemand,
				OutputMode::Log,
				DUST_EXPECTED_FILES.to_owned(),
			)
			.expect("resolver could not be created")
		),
		Box::new(|_key_location| Box::pin(std::future::ready(Ok(None)))),
	);
}

pub struct LedgerContext<D: DB + Clone> {
	pub ledger_state: Mutex<LedgerState<D>>,
	pub latest_block_context: Mutex<Option<BlockContext>>,
	pub wallets: Mutex<HashMap<WalletSeed, Wallet<D>>>,
	pub resolver: MutexTokio<&'static Resolver>,
}

#[derive(Debug, Storable)]
#[derive_where(Clone)]
#[storable(db = D)]
struct StorableLedgerState<D: DB> {
	state: LedgerState<D>,
	block_fullness: StorableSyntheticCost<D>,
}

impl<D: DB> StorableLedgerState<D> {
	fn new(state: LedgerState<D>) -> Self {
		Self { state, block_fullness: StorableSyntheticCost::zero() }
	}
}

impl<D: DB> Tagged for StorableLedgerState<D> {
	fn tag() -> std::borrow::Cow<'static, str> {
		<LedgerState<D> as Tagged>::tag()
	}

	fn tag_unique_factor() -> String {
		<LedgerState<D> as Tagged>::tag_unique_factor()
	}
}

impl<D: DB + Clone> LedgerContext<D> {
	pub fn new(network_id: impl Into<String>) -> Self {
		Self {
			ledger_state: Mutex::new(LedgerState::new(network_id)),
			wallets: Mutex::new(HashMap::new()),
			resolver: MutexTokio::new(&DEFAULT_RESOLVER),
			latest_block_context: Mutex::new(None),
		}
	}

	pub fn new_from_wallet_seeds(
		network_id: impl Into<String>,
		wallet_seeds: &[WalletSeed],
	) -> Self {
		let ledger_state = LedgerState::new(network_id);
		let wallets = Mutex::new(HashMap::new());

		// Use default `Resolver` for Zswaps
		let resolver = MutexTokio::new(&*DEFAULT_RESOLVER);

		for seed in wallet_seeds {
			let wallet = Wallet::default(*seed, &ledger_state);
			wallets
				.lock()
				.expect("Error locking `LedgerContext` wallets")
				.insert(*seed, wallet);
		}

		Self {
			ledger_state: Mutex::new(ledger_state),
			wallets,
			resolver,
			latest_block_context: Mutex::new(None),
		}
	}

	pub fn update_from_block<S: SignatureKind<D>, P: ProofKind<D> + std::fmt::Debug>(
		&self,
		txs: Vec<SerdeTransaction<S, P, D>>,
		block_context: BlockContext,
		state_root: Option<Vec<u8>>,
	) where
		Transaction<S, P, PureGeneratorPedersen, D>: Tagged,
	{
		// Capture pre-block state for debugging (before any mutations)
		let pre_block_state: Option<Vec<u8>> = if state_root.is_some()
			&& block_context.parent_block_hash.0
				== hex_literal::hex!(
					"51eea6988210397f9de4bff178657f1a7bd8fdf7215f9072530aed25e542640d"
				) {
			self.ledger_state.lock().ok().and_then(|s| super::serialize(&*s).ok())
		} else {
			None
		};

		let mut total_cost = SyntheticCost::ZERO;
		for tx in &txs {
			let (events, cost) = self.update_from_tx(tx, &block_context);
			for wallet in
				self.wallets.lock().expect("Error locking `LedgerContext` wallets").values_mut()
			{
				wallet.update_dust_from_tx(&events).unwrap_or_else(|e| {
					panic!(
						"failed to replay dust events for tx {}: {e}",
						tx.transaction_hash().0.0.encode_hex::<String>()
					)
				});
			}
			total_cost = total_cost + cost;
		}

		// Only when done processing txs for the same block, it's time to call `post_block_update`
		let mut latest_ledger_state =
			self.ledger_state.lock().expect("Error locking `LedgerContext` ledger_state");
		let block_limits = latest_ledger_state.parameters.limits.block_limits;
		let normalized_fullness =
			total_cost.normalize(block_limits).unwrap_or(NormalizedCost::ZERO);
		let overall_fullness = compute_overall_fullness(&normalized_fullness);
		*latest_ledger_state = latest_ledger_state
			.post_block_update(block_context.tblock, normalized_fullness, overall_fullness)
			.expect("Error applying block updates");
		if let Some(expected_root) = state_root {
			let actual_root = Self::compute_state_root(&*latest_ledger_state);

			// Log every block's state roots for comparison across runs (diagnostic for
			// intermittent state root mismatch bug)
			if let Ok(mut file) =
				OpenOptions::new().create(true).append(true).open("/tmp/state_roots.log")
			{
				let _ = writeln!(
					file,
					"block={}, expected={}, actual={}, parent={}, tx_count={}",
					block_context.tblock.to_secs(),
					hex_encode(&expected_root),
					actual_root
						.as_ref()
						.map(|r| hex_encode(r))
						.unwrap_or_else(|| "NONE".to_string()),
					hex_encode(block_context.parent_block_hash.0),
					txs.len(),
				);
			}

			match actual_root {
				Some(actual) if actual != expected_root => {
					// Dump reproducible test data
					Self::dump_mismatch_data(
						&pre_block_state,
						&txs,
						&block_context,
						&expected_root,
						&actual,
					);
					panic!(
						"Ledger state root mismatch: expected {}, actual {}. Parent block hash: {}. Test data dumped to /tmp/state_mismatch/",
						hex_encode(&expected_root),
						hex_encode(&actual),
						hex_encode(block_context.parent_block_hash.0),
					);
				},
				Some(_) => {},
				None => println!("Failed to compute local ledger state root for comparison"),
			}
		}
		// Update Local Wallets
		for wallet in
			self.wallets.lock().expect("Error locking `LedgerContext` wallets").values_mut()
		{
			wallet.update_dust_from_block(&block_context);
		}
		// Update latest block context
		*self.latest_block_context.lock().expect("error locking latest_block_context") =
			Some(block_context.clone());
	}

	pub fn latest_block_context(&self) -> BlockContext {
		self.latest_block_context
			.lock()
			.expect("failed to lock latest_block_context")
			.as_ref()
			.cloned()
			.unwrap_or_else(|| {
				let now = Timestamp::from_secs(
					SystemTime::now()
						.duration_since(UNIX_EPOCH)
						.expect("time has run backwards")
						.as_secs(),
				);
				BlockContext { tblock: now, tblock_err: 30, parent_block_hash: Default::default() }
			})
	}

	pub fn compute_state_root(state: &LedgerState<D>) -> Option<Vec<u8>> {
		let storage = default_storage::<D>();
		let ledger = StorableLedgerState::new(state.clone());
		let sp = storage.arena.alloc(ledger);
		super::serialize(&sp.as_typed_key()).ok()
	}

	/// Dump all data needed to reproduce a state root mismatch for debugging.
	fn dump_mismatch_data<S: SignatureKind<D>, P: ProofKind<D>>(
		pre_block_state: &Option<Vec<u8>>,
		txs: &[SerdeTransaction<S, P, D>],
		block_context: &BlockContext,
		expected_root: &[u8],
		actual_root: &[u8],
	) where
		Transaction<S, P, PureGeneratorPedersen, D>: Tagged,
	{
		use std::fs;

		let dir = "/tmp/state_mismatch";
		fs::create_dir_all(dir).expect("Failed to create mismatch dump dir");

		// Dump pre-block ledger state
		if let Some(state_bytes) = pre_block_state {
			fs::write(format!("{dir}/pre_block_state.bin"), state_bytes)
				.expect("Failed to write pre_block_state.bin");
		}

		// Dump transactions
		for (i, tx) in txs.iter().enumerate() {
			let tx_bytes = tx.serialize_inner().expect("Failed to serialize tx");
			fs::write(format!("{dir}/tx_{i}.bin"), &tx_bytes).expect("Failed to write tx bin");
		}

		// Dump block context
		let ctx_bytes = super::serialize(block_context).expect("Failed to serialize block_context");
		fs::write(format!("{dir}/block_context.bin"), &ctx_bytes)
			.expect("Failed to write block_context.bin");

		// Dump human-readable summary
		let summary = format!(
			"tblock_secs={}\ntblock_err={}\nparent_block_hash={}\nexpected_root={}\nactual_root={}\ntx_count={}\n",
			block_context.tblock.to_secs(),
			block_context.tblock_err,
			hex_encode(block_context.parent_block_hash.0),
			hex_encode(expected_root),
			hex_encode(actual_root),
			txs.len(),
		);
		fs::write(format!("{dir}/summary.txt"), &summary).expect("Failed to write summary.txt");

		// Dump expected and actual roots
		fs::write(format!("{dir}/expected_root.bin"), expected_root)
			.expect("Failed to write expected_root.bin");
		fs::write(format!("{dir}/actual_root.bin"), actual_root)
			.expect("Failed to write actual_root.bin");

		println!("Mismatch data dumped to {dir}/");
	}

	pub fn update_from_tx<S: SignatureKind<D>, P: ProofKind<D> + std::fmt::Debug>(
		&self,
		tx: &SerdeTransaction<S, P, D>,
		block_context: &BlockContext,
	) -> (Vec<Event<D>>, SyntheticCost)
	where
		Transaction<S, P, PureGeneratorPedersen, D>: Tagged,
	{
		let tx_context = self.tx_context(block_context.clone());

		let strictness: WellFormedStrictness =
			if block_context.parent_block_hash == Default::default() {
				let mut lax: WellFormedStrictness = Default::default();
				lax.enforce_balancing = false;
				lax
			} else {
				Default::default()
			};

		// Update Ledger State
		let (new_ledger_state, offers, events, cost) = match &tx {
			SerdeTransaction::Midnight(tx) => {
				let valid_tx: VerifiedTransaction<_> = tx
					.well_formed(&tx_context.ref_state, strictness, tx_context.block_context.tblock)
					.expect("applying invalid transaction");
				let cost = tx
					.cost(&tx_context.ref_state.parameters, false)
					.expect("error calculating fees");

				let (new_ledger_state, result) = tx_context.ref_state.apply(&valid_tx, &tx_context);
				let offers = Self::successful_shielded_offers(tx, &result);
				match result {
					TransactionResult::Success(events) => (new_ledger_state, offers, events, cost),
					TransactionResult::PartialSuccess(failure, events) => {
						let hash = hex::encode(tx.transaction_hash().0.0);
						println!(
							"Partially failing result {failure:?} of applying tx 0x{hash} to update Local Ledger State"
						);
						(new_ledger_state, offers, events, cost)
					},
					TransactionResult::Failure(failure) => {
						let hash = hex::encode(tx.transaction_hash().0.0);
						println!(
							"Failing result {failure:?} of applying tx 0x{hash} \nto update Local Ledger State"
						);
						(new_ledger_state, offers, vec![], SyntheticCost::ZERO)
					},
				}
			},
			SerdeTransaction::System(tx) => {
				let cost = tx.cost(&tx_context.ref_state.parameters);
				match tx_context.ref_state.apply_system_tx(tx, block_context.tblock) {
					Ok((new_state, events)) => (new_state, vec![], events, cost),
					Err(err) => {
						let hash = hex::encode(tx.transaction_hash().0.0);
						println!(
							"Failing result {err:?} of applying system tx {hash} to update Local Ledger State"
						);
						(tx_context.ref_state.clone(), vec![], vec![], cost)
					},
				}
			},
		};

		// Update Local Wallets
		for wallet in
			self.wallets.lock().expect("Error locking `LedgerContext` wallets").values_mut()
		{
			wallet.update_state_from_offers(&offers);
		}

		*self.ledger_state.lock().expect("Error locking `LedgerContext` ledger_state") =
			new_ledger_state;
		(events, cost)
	}

	fn successful_shielded_offers<S: SignatureKind<D>, P: ProofKind<D>>(
		tx: &Transaction<S, P, PureGeneratorPedersen, D>,
		result: &TransactionResult<D>,
	) -> Vec<Offer<P::LatestProof, D>> {
		let failed_segments = match result {
			TransactionResult::Success(_) => HashSet::new(),
			TransactionResult::Failure(_) => return vec![],
			TransactionResult::PartialSuccess(results, _) => {
				let mut failures = HashSet::new();
				for (segment, result) in results {
					if result.is_err() {
						failures.insert(segment);
					}
				}
				failures
			},
		};
		let Transaction::Standard(stx) = tx else {
			return vec![];
		};
		let mut offers = vec![];
		if let Some(guaranteed) = &stx.guaranteed_coins {
			offers.push((**guaranteed).clone());
		}
		for entry in stx.fallible_coins.iter() {
			let segment = *entry.0;
			let fallible = &entry.1;
			if !failed_segments.contains(&segment) {
				offers.push((**fallible).clone());
			}
		}
		offers
	}

	pub fn utxos(&self, address: WalletAddress) -> Vec<Utxo> {
		self.ledger_state
			.lock()
			.expect("Error locking `LedgerContext` ledger_state")
			.utxo
			.utxos
			.iter()
			.filter(|utxo| &utxo.0.owner.0.0.to_vec() == address.data())
			.map(|utxo| (*utxo.0).clone())
			.collect::<Vec<_>>()
	}

	pub async fn update_resolver(&self, resolver: &'static Resolver) {
		let mut resolver_guard = self.resolver.lock().await;

		*resolver_guard = resolver
	}

	pub async fn resolver(&self) -> &Resolver {
		self.resolver.lock().await.clone()
	}

	pub fn wallet_from_seed(&self, seed: WalletSeed) -> Wallet<D> {
		let mut wallet_guard = self.wallets.lock().expect("Error locking `LedgerContext` wallets");
		let wallet = Self::wallet_for_seed(&mut wallet_guard, seed);

		Wallet {
			root_seed: wallet.root_seed,
			shielded: wallet.shielded.clone(),
			unshielded: wallet.unshielded.clone(),
			dust: wallet.dust.clone(),
		}
	}

	/// Helper to get or create a wallet for a seed within an existing lock.
	fn wallet_for_seed(
		wallets: &mut HashMap<WalletSeed, Wallet<D>>,
		seed: WalletSeed,
	) -> &mut Wallet<D> {
		wallets.get_mut(&seed).unwrap_or_else(|| {
			panic!("Wallet with seed {seed:?} does not exists in the `LedgerContext")
		})
	}

	/// Operate on a single wallet identified by seed.
	pub fn with_wallet_from_seed<F, R>(&self, seed: WalletSeed, f: F) -> R
	where
		F: FnOnce(&mut Wallet<D>) -> R,
	{
		let mut wallet_guard = self.wallets.lock().expect("Error locking `LedgerContext` wallets");
		let wallet = Self::wallet_for_seed(&mut wallet_guard, seed);
		f(wallet)
	}

	/// Operate on two wallets identified by origin and destination seeds.
	pub fn with_wallets_from_seeds<F, R>(
		&self,
		origin_seed: WalletSeed,
		destination_seed: WalletSeed,
		f: F,
	) -> R
	where
		F: FnOnce(&mut Wallet<D>, &mut Wallet<D>) -> R,
	{
		assert_ne!(origin_seed, destination_seed, "Origin and destination seeds must be different");

		let mut wallet_guard = self.wallets.lock().expect("Error locking `LedgerContext` wallets");

		// Remove both wallets temporarily to get mutable references without borrowing issues
		let mut origin_wallet = wallet_guard
			.remove(&origin_seed)
			.unwrap_or_else(|| panic!("Wallet with seed {:?} does not exist", origin_seed));
		let mut destination_wallet = wallet_guard
			.remove(&destination_seed)
			.unwrap_or_else(|| panic!("Wallet with seed {:?} does not exist", destination_seed));

		let result = f(&mut origin_wallet, &mut destination_wallet);

		// Reinsert wallets
		wallet_guard.insert(origin_seed, origin_wallet);
		wallet_guard.insert(destination_seed, destination_wallet);

		result
	}

	pub fn with_ledger_state<F, R>(&self, f: F) -> R
	where
		F: FnOnce(&mut LedgerState<D>) -> R,
	{
		let mut ledger_state =
			self.ledger_state.lock().expect("Error locking `LedgerContext` ledger_state");
		f(&mut ledger_state)
	}

	pub fn tx_context(&self, block_context: BlockContext) -> TransactionContext<D> {
		self.with_ledger_state(|ledger_state| TransactionContext {
			ref_state: ledger_state.clone(),
			block_context,
			whitelist: None,
		})
	}
}

