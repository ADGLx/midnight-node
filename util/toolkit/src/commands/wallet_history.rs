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

use crate::source::Source;
use crate::tx_generator::builder::build_fork_aware_context_cached;
use crate::tx_generator::source::create_file_wallet_cache;
use crate::{HRP_CREDENTIAL_SHIELDED, TxGenerator, WalletAddress, WalletSeed};
use crate::cli_parsers as cli;
use clap::Args;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletTransactionRecord {
	pub block_number: u64,
	#[serde(with = "hex")]
	pub transaction_hash: [u8; 32],
	pub transaction_type: String,
	pub timestamp_secs: u64,
	pub changes: TransactionChanges,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionChanges {
	pub coins_added: usize,
	pub coins_spent: usize,
	pub utxos_added: usize,
	pub utxos_spent: usize,
	pub dust_outputs_added: usize,
	pub dust_outputs_spent: usize,
}

#[derive(Debug, Serialize)]
pub struct WalletTransactionHistory {
	pub transactions: Vec<WalletTransactionRecord>,
}

#[derive(Debug)]
pub enum WalletHistoryResult {
	Json(WalletTransactionHistory),
	DryRun(()),
}

#[derive(Args)]
#[group(id = "wallet_id", required = true, multiple = false)]
pub struct WalletHistoryArgs {
	#[command(flatten)]
	source: Source,
	/// The seed of the wallet to show transaction history for
	#[arg(long, value_parser = cli::wallet_seed_decode, group = "wallet_id")]
	seed: Option<WalletSeed>,
	/// The address of the wallet to show transaction history for (unshielded only)
	#[arg(long, value_parser = cli::wallet_address, group = "wallet_id")]
	address: Option<WalletAddress>,
	/// Dry-run - don't fetch wallet history, just print out settings
	#[arg(long)]
	dry_run: bool,
}

pub async fn execute(
	args: WalletHistoryArgs,
) -> Result<WalletHistoryResult, Box<dyn std::error::Error + Send + Sync>> {
	let ledger_state_db = args.source.ledger_state_db.clone();
	let fetch_cache = args.source.fetch_cache.clone();
	let src = TxGenerator::source(args.source, args.dry_run).await?;

	if args.dry_run {
		if let Some(seed) = args.seed {
			log::info!("Dry-run: fetching transaction history for seed {:?}", seed);
		} else {
			log::info!(
				"Dry-run: fetching transaction history for address {:?}",
				args.address.unwrap()
			);
		}
		return Ok(WalletHistoryResult::DryRun(()));
	}

	let source_blocks = src.get_txs().await?;
	let wallet_cache = create_file_wallet_cache(&ledger_state_db, &fetch_cache);

	if let Some(seed) = args.seed {
		let fork_ctx =
			build_fork_aware_context_cached(&[seed], &source_blocks, wallet_cache.as_deref()).await;

		Ok(fork_ctx.dispatch(
			|ctx| {
				let seed_v7 =
					crate::tx_generator::builder::builders::ledger_7::type_convert::convert_wallet_seed(seed);
				let result = crate::commands::fork::ledger_7::wallet_history::get_wallet_history_from_seed(
					&ctx,
					seed_v7,
					&source_blocks.blocks,
				);
				fork_history_result_v7(result)
			},
			|ctx| {
				let result = crate::commands::fork::ledger_8::wallet_history::get_wallet_history_from_seed(
					&ctx,
					seed,
					&source_blocks.blocks,
				);
				fork_history_result_v8(result)
			},
		))
	} else {
		let address = args.address.expect("parsing error; address not given");
		if address.human_readable_part().contains(HRP_CREDENTIAL_SHIELDED) {
			return Err("transaction history unavailable for shielded addresses - use seed instead"
				.into());
		}

		let fork_ctx =
			build_fork_aware_context_cached(&[], &source_blocks, wallet_cache.as_deref()).await;

		let address_clone = address.clone();
		Ok(fork_ctx.dispatch(
			|ctx| {
				let addr_v7 =
					crate::tx_generator::builder::builders::ledger_7::type_convert::convert_wallet_address(
						&address_clone,
					);
				let result = crate::commands::fork::ledger_7::wallet_history::get_wallet_history_from_address(
					&ctx,
					addr_v7,
					&source_blocks.blocks,
				);
				fork_history_result_v7(result)
			},
			|ctx| {
				let result = crate::commands::fork::ledger_8::wallet_history::get_wallet_history_from_address(
					&ctx,
					address,
					&source_blocks.blocks,
				);
				fork_history_result_v8(result)
			},
		))
	}
}

fn fork_history_result_v8(
	result: crate::commands::fork::ledger_8::wallet_history::WalletHistoryResult,
) -> WalletHistoryResult {
	WalletHistoryResult::Json(WalletTransactionHistory {
		transactions: result.transactions,
	})
}

fn fork_history_result_v7(
	result: crate::commands::fork::ledger_7::wallet_history::WalletHistoryResult,
) -> WalletHistoryResult {
	WalletHistoryResult::Json(WalletTransactionHistory {
		transactions: result.transactions,
	})
}
