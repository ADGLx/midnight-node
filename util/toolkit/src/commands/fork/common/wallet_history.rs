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

use super::ledger_helpers_local::{self, DefaultDB, WalletAddress, WalletSeed};
use crate::commands::wallet_history::{TransactionChanges, WalletTransactionRecord};
use midnight_node_ledger_helpers::fork::raw_block_data::RawBlockData;

pub struct WalletHistoryResult {
	pub transactions: Vec<WalletTransactionRecord>,
}

pub fn get_wallet_history_from_seed(
	_context: &ledger_helpers_local::context::LedgerContext<DefaultDB>,
	_seed: WalletSeed,
	blocks: &[RawBlockData],
) -> WalletHistoryResult {
	// Simplified implementation: iterate through blocks and extract transaction info
	// This is a basic version that doesn't track actual wallet state changes
	// A full implementation would need to replay transactions and track state

	let mut transaction_history = Vec::new();

	for block in blocks {
		// For each transaction in the block, add a record
		// In a real implementation, we would check if the transaction affects this wallet
		for (tx_index, raw_tx) in block.transactions.iter().enumerate() {
			let tx_hash = compute_tx_hash(raw_tx, tx_index);

			// Placeholder changes - a real implementation would track actual state changes
			let changes = TransactionChanges {
				coins_added: 0,
				coins_spent: 0,
				utxos_added: 0,
				utxos_spent: 0,
				dust_outputs_added: 0,
				dust_outputs_spent: 0,
			};

			transaction_history.push(WalletTransactionRecord {
				block_number: block.number,
				transaction_hash: tx_hash,
				transaction_type: classify_transaction_type(raw_tx),
				timestamp_secs: block.tblock_secs,
				changes,
			});
		}
	}

	WalletHistoryResult { transactions: transaction_history }
}

pub fn get_wallet_history_from_address(
	_context: &ledger_helpers_local::context::LedgerContext<DefaultDB>,
	_address: WalletAddress,
	blocks: &[RawBlockData],
) -> WalletHistoryResult {
	let mut transaction_history = Vec::new();

	// Simplified implementation
	for block in blocks {
		for (tx_index, raw_tx) in block.transactions.iter().enumerate() {
			let tx_hash = compute_tx_hash(raw_tx, tx_index);

			let changes = TransactionChanges {
				coins_added: 0,
				coins_spent: 0,
				utxos_added: 0,
				utxos_spent: 0,
				dust_outputs_added: 0,
				dust_outputs_spent: 0,
			};

			transaction_history.push(WalletTransactionRecord {
				block_number: block.number,
				transaction_hash: tx_hash,
				transaction_type: classify_transaction_type(raw_tx),
				timestamp_secs: block.tblock_secs,
				changes,
			});
		}
	}

	WalletHistoryResult { transactions: transaction_history }
}

fn compute_tx_hash(
	raw_tx: &midnight_node_ledger_helpers::fork::raw_block_data::RawTransaction,
	tx_index: usize,
) -> [u8; 32] {
	use sp_crypto_hashing::blake2_256;

	let bytes = match raw_tx {
		midnight_node_ledger_helpers::fork::raw_block_data::RawTransaction::Midnight(b) => b,
		midnight_node_ledger_helpers::fork::raw_block_data::RawTransaction::System(b) => b,
	};

	// Combine transaction bytes with index for uniqueness
	let mut combined = Vec::with_capacity(bytes.len() + 8);
	combined.extend_from_slice(bytes);
	combined.extend_from_slice(&tx_index.to_le_bytes());

	blake2_256(&combined)
}

fn classify_transaction_type(
	raw_tx: &midnight_node_ledger_helpers::fork::raw_block_data::RawTransaction,
) -> String {
	match raw_tx {
		midnight_node_ledger_helpers::fork::raw_block_data::RawTransaction::Midnight(_) => {
			"midnight".to_string()
		},
		midnight_node_ledger_helpers::fork::raw_block_data::RawTransaction::System(_) => {
			"system".to_string()
		},
	}
}
