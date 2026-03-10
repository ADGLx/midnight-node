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

use std::{collections::HashMap, io::Write, sync::Arc};

use super::ledger_helpers_local::{
	BuildIntent, BuildUtxoOutput, BuildUtxoSpend, DefaultDB, FromContext as _, IntentInfo,
	LedgerContext, ProofProvider, Segment, StandardTrasactionInfo, TransactionWithContext,
	UnshieldedOfferInfo, UnshieldedTokenType, UnshieldedWallet, UtxoOutputInfo, UtxoSpendInfo,
	WalletAddress, WalletSeed,
};
use async_trait::async_trait;
use tokio::sync::Semaphore;

use crate::{
	Progress, serde_def::SourceTransactions, tx_generator::builder::BatchSingleTxArgs,
};
use midnight_node_ledger_helpers::fork::raw_block_data::SerializedTxBatches;

use crate::tx_generator::builder::{BuildTxs, TransferSpec};

pub struct BatchSingleTxBuilder {
	context: Arc<LedgerContext<DefaultDB>>,
	prover: Arc<dyn ProofProvider<DefaultDB>>,
	transfers: Vec<TransferSpec>,
	concurrency: usize,
}

impl BatchSingleTxBuilder {
	pub fn new(
		args: BatchSingleTxArgs,
		context: Arc<LedgerContext<DefaultDB>>,
		prover: Arc<dyn ProofProvider<DefaultDB>>,
	) -> Self {
		let transfers: Vec<TransferSpec> = {
			let file_content = std::fs::read_to_string(&args.transfers_file)
				.unwrap_or_else(|e| panic!("failed to read transfers file '{}': {}", args.transfers_file, e));
			serde_json::from_str(&file_content)
				.unwrap_or_else(|e| panic!("failed to parse transfers JSON: {}", e))
		};

		let concurrency = args
			.concurrency
			.unwrap_or_else(|| std::thread::available_parallelism().unwrap().into());

		Self { context, prover, transfers, concurrency }
	}

	fn build_single_transfer(
		context: Arc<LedgerContext<DefaultDB>>,
		prover: Arc<dyn ProofProvider<DefaultDB>>,
		spec: &TransferSpec,
	) -> TransactionWithContext<
		super::ledger_helpers_local::Signature,
		super::ledger_helpers_local::ProofMarker,
		DefaultDB,
	> {
		use super::type_convert::*;

		let source_seed = convert_wallet_seed(
			spec.source_seed.parse().expect("invalid source_seed hex"),
		);
		let funding_seed = spec
			.funding_seed
			.as_ref()
			.map(|s| convert_wallet_seed(s.parse().expect("invalid funding_seed hex")))
			.unwrap_or(source_seed);

		let rng_seed: Option<[u8; 32]> = spec.rng_seed.as_ref().map(|s| {
			let bytes = hex::decode(s).expect("invalid rng_seed hex");
			bytes.try_into().expect("rng_seed must be 32 bytes")
		});

		let dest_address: WalletAddress = convert_wallet_address(
			&spec
				.destination_address
				.parse()
				.expect("invalid destination_address"),
		);

		let mut tx_info =
			StandardTrasactionInfo::new_from_context(context.clone(), prover, rng_seed);

		if let Some(amount) = spec.unshielded_amount {
			let token_type_str = spec.unshielded_token_type.as_deref().unwrap_or(
				"0000000000000000000000000000000000000000000000000000000000000000",
			);
			let token_type: midnight_node_ledger_helpers::UnshieldedTokenType =
				midnight_node_ledger_helpers::UnshieldedTokenType(
					midnight_node_ledger_helpers::HashOutput(
						hex::decode(token_type_str)
							.expect("invalid unshielded_token_type hex")
							.try_into()
							.expect("unshielded_token_type must be 32 bytes"),
					),
				);
			let token_type: UnshieldedTokenType = convert_unshielded_token_type(token_type);

			let dest_wallet: UnshieldedWallet =
				(&dest_address).try_into().expect("destination is not a valid unshielded address");

			let intents = build_unshielded_intents(
				context.clone(),
				source_seed,
				vec![dest_wallet],
				amount,
				token_type,
			);
			tx_info.set_intents(intents);
		}

		if let Some(amount) = spec.shielded_amount {
			use super::ledger_helpers_local::{
				BuildInput, BuildOutput, InputInfo, OfferInfo, OutputInfo, ShieldedTokenType,
				ShieldedWallet,
			};

			let token_type_str = spec.shielded_token_type.as_deref().unwrap_or(
				"0000000000000000000000000000000000000000000000000000000000000000",
			);
			let token_type: midnight_node_ledger_helpers::ShieldedTokenType =
				midnight_node_ledger_helpers::ShieldedTokenType(
					midnight_node_ledger_helpers::HashOutput(
						hex::decode(token_type_str)
							.expect("invalid shielded_token_type hex")
							.try_into()
							.expect("shielded_token_type must be 32 bytes"),
					),
				);
			let token_type: ShieldedTokenType = convert_shielded_token_type(token_type);

			let dest_wallet: ShieldedWallet<DefaultDB> =
				(&dest_address).try_into().expect("destination is not a valid shielded address");

			let input_info = InputInfo {
				origin: source_seed,
				token_type,
				value: amount,
			};

			let inputs_info: Vec<Box<dyn BuildInput<DefaultDB>>> = vec![Box::new(input_info)];

			let output_info: Box<dyn BuildOutput<DefaultDB>> = Box::new(OutputInfo {
				destination: dest_wallet,
				token_type,
				value: amount,
			});

			let funding_wallet = context.clone().wallet_from_seed(source_seed);
			let input_amount = input_info.min_match_coin(&funding_wallet.shielded.state).value;
			let remaining = input_amount.checked_sub(amount).expect("insufficient shielded input");

			let mut outputs_info: Vec<Box<dyn BuildOutput<DefaultDB>>> = vec![output_info];
			outputs_info.push(Box::new(OutputInfo {
				destination: source_seed,
				token_type,
				value: remaining,
			}));

			let offer = OfferInfo {
				inputs: inputs_info,
				outputs: outputs_info,
				transients: vec![],
			};

			if offer.outputs.len() > 2 {
				tx_info.set_fallible_offers(HashMap::from([(1, offer)]));
			} else {
				tx_info.set_guaranteed_offer(offer);
			}
		}

		tx_info.set_funding_seeds(vec![funding_seed]);
		tx_info.use_mock_proofs_for_fees(true);

		if tx_info.is_empty() {
			panic!(
				"transfer to {} is empty — must specify shielded_amount or unshielded_amount",
				spec.destination_address
			);
		}

		let tx = tokio::runtime::Handle::current()
			.block_on(tx_info.prove())
			.expect("Balancing TX failed");

		TransactionWithContext::new(tx, None)
	}
}

const MAX_GUARANTEED_INPUTS_OUTPUTS: usize = 3;

fn build_unshielded_intents(
	context: Arc<LedgerContext<DefaultDB>>,
	source_seed: WalletSeed,
	output_wallets: Vec<UnshieldedWallet>,
	amount_per_output: u128,
	token_type: UnshieldedTokenType,
) -> HashMap<u16, Box<dyn BuildIntent<DefaultDB>>> {
	let total_required = amount_per_output
		.checked_mul(output_wallets.len() as u128)
		.expect("unshielded amount overflow");

	let (inputs_info, remaining) = UtxoSpendInfo::utxos_to_cover_value(
		context,
		source_seed,
		total_required,
		token_type,
	);

	let inputs_info: Vec<Box<dyn BuildUtxoSpend<DefaultDB>>> = inputs_info
		.into_iter()
		.map(|input| {
			let input: Box<dyn BuildUtxoSpend<DefaultDB>> = Box::new(input);
			input
		})
		.collect();

	let mut outputs_info: Vec<Box<dyn BuildUtxoOutput<DefaultDB>>> = output_wallets
		.iter()
		.map(|wallet| {
			let output: Box<dyn BuildUtxoOutput<DefaultDB>> = Box::new(UtxoOutputInfo {
				value: amount_per_output,
				owner: wallet.clone(),
				token_type,
			});
			output
		})
		.collect();

	if remaining > 0 {
		outputs_info.push(Box::new(UtxoOutputInfo {
			value: remaining,
			owner: source_seed,
			token_type,
		}));
	}

	let inputs_outputs_len = inputs_info.len() + outputs_info.len();
	let unshielded_offer = UnshieldedOfferInfo { inputs: inputs_info, outputs: outputs_info };

	let intent_info = if inputs_outputs_len > MAX_GUARANTEED_INPUTS_OUTPUTS {
		IntentInfo {
			guaranteed_unshielded_offer: None,
			fallible_unshielded_offer: Some(unshielded_offer),
			actions: vec![],
		}
	} else {
		IntentInfo {
			guaranteed_unshielded_offer: Some(unshielded_offer),
			fallible_unshielded_offer: None,
			actions: vec![],
		}
	};

	let mut intents = HashMap::new();
	intents.insert(Segment::Fallible.into(), Box::new(intent_info) as Box<dyn BuildIntent<DefaultDB>>);
	intents
}

#[async_trait]
impl BuildTxs for BatchSingleTxBuilder {
	type Error = BatchSingleTxError;

	async fn build_txs_from(
		&self,
		_received_tx: SourceTransactions,
	) -> Result<SerializedTxBatches, Self::Error> {
		let total = self.transfers.len();
		log::info!("Building {} transfers from batch spec...", total);

		let progress = Progress::new(total, "generating batch-single-tx transfers");
		let sema = Arc::new(Semaphore::new(self.concurrency));

		let tasks: Vec<_> = self
			.transfers
			.iter()
			.map(|spec| {
				let context = self.context.clone();
				let prover = self.prover.clone();
				let spec = spec.clone();
				let sema = sema.clone();

				tokio::task::spawn_blocking(move || {
					let rt = tokio::runtime::Handle::current();
					let _permit = rt.block_on(async { sema.acquire().await.unwrap() });
					let tx_with_ctx = Self::build_single_transfer(context, prover, &spec);
					let serialized = super::tx_serialization::build_single(tx_with_ctx);
					let tx = serialized
						.batches
						.into_iter()
						.next()
						.and_then(|b| b.into_iter().next())
						.expect("build_single should produce exactly one tx");

					if let Some(parent) = std::path::Path::new(&spec.dest_file).parent() {
						std::fs::create_dir_all(parent).unwrap_or_else(|e| {
							panic!("failed to create directory for '{}': {}", spec.dest_file, e)
						});
					}
					let mut file = std::fs::File::create(&spec.dest_file).unwrap_or_else(|e| {
						panic!("failed to create dest_file '{}': {}", spec.dest_file, e)
					});
					file.write_all(&serde_json::to_vec(&tx).expect("serialization error"))
						.unwrap_or_else(|e| {
							panic!("failed to write to '{}': {}", spec.dest_file, e)
						});

					spec.dest_file.clone()
				})
			})
			.collect();

		let mut succeeded = 0usize;
		let mut failed = 0usize;

		for task in tasks {
			match task.await {
				Ok(dest_file) => {
					log::info!("Wrote tx to {}", dest_file);
					succeeded += 1;
				},
				Err(e) => {
					log::error!("Transfer failed: {}", e);
					failed += 1;
				},
			}
			progress.inc(1);
		}

		progress.finish(format!("batch-single-tx: {} succeeded, {} failed", succeeded, failed));

		if failed > 0 {
			return Err(BatchSingleTxError::PartialFailure { succeeded, failed });
		}

		// Txs already written to individual files — return empty batches
		Ok(SerializedTxBatches { batches: vec![] })
	}
}

#[derive(Debug, thiserror::Error)]
pub enum BatchSingleTxError {
	#[error("{failed} of {} transfers failed", succeeded + failed)]
	PartialFailure { succeeded: usize, failed: usize },
}
