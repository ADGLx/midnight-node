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

use serde::{Deserialize, Serialize};
use std::{
	fmt::Debug,
	time::{SystemTime, UNIX_EPOCH},
};
use subxt::utils::H256;

use crate::fetcher::fetch_storage::BlockData;
use midnight_node_ledger_helpers::*;

#[derive(Clone, Debug)]
pub struct SourceTransactions<S: SignatureKind<D> + Tagged, P: ProofKind<D>, D: DB + Clone>
where
	Transaction<S, P, PureGeneratorPedersen, D>: Tagged,
{
	pub blocks: Vec<BlockData<S, P, D>>,
}

impl<S: SignatureKind<D> + Tagged, P: ProofKind<D>, D: DB + Clone> SourceTransactions<S, P, D>
where
	Transaction<S, P, PureGeneratorPedersen, D>: Tagged,
{
	pub fn from_txs_with_context(
		txs: impl IntoIterator<Item = TransactionWithContext<S, P, D>>,
		dust_warp: bool,
	) -> Self {
		let mut blocks = vec![];
		let mut current_batch = vec![];
		let mut last_context: Option<BlockContext> = None;
		let mut number = 0;
		for tx in txs {
			if last_context.as_ref().is_some_and(|c| c.tblock != tx.block_context.tblock) {
				blocks.push(BlockData {
					hash: H256::zero(),
					parent_hash: H256::zero(),
					number,
					transactions: std::mem::take(&mut current_batch),
					context: last_context.unwrap(),
					state_root: None,
				});
				number += 1;
			}
			current_batch.push(tx.tx);
			last_context = Some(tx.block_context);
		}
		if let Some(context) = last_context {
			blocks.push(BlockData {
				hash: H256::zero(),
				parent_hash: H256::zero(),
				number,
				transactions: current_batch,
				context,
				state_root: None,
			});
		}

		if dust_warp {
			// Add an empty block with a now() as a block_context
			let now = Timestamp::from_secs(
				SystemTime::now()
					.duration_since(UNIX_EPOCH)
					.expect("time has run backwards")
					.as_secs(),
			);
			let context =
				BlockContext { tblock: now, tblock_err: 30, parent_block_hash: Default::default() };
			blocks.push(BlockData {
				hash: H256::zero(),
				parent_hash: H256::zero(),
				number: 0,
				transactions: Vec::new(),
				context,
				state_root: None,
			});
		}
		Self { blocks }
	}

	pub fn network(&self) -> &str {
		self.blocks
			.iter()
			.find_map(|b| b.transactions.iter().find_map(|tx| tx.network_id()))
			.expect("no transaction in this batch had a network")
	}
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SourceBlockTransactions<S: SignatureKind<D>, P: ProofKind<D>, D: DB + Clone>
where
	Transaction<S, P, PureGeneratorPedersen, D>: Tagged,
{
	#[serde(bound = "")]
	pub transactions: Vec<SerdeTransaction<S, P, D>>,
	pub context: BlockContext,
	#[serde(default)]
	pub state_root: Option<Vec<u8>>,
}

#[derive(Clone, Debug)]
pub struct DeserializedTransactionsWithContextBatch<
	S: SignatureKind<D>,
	P: ProofKind<D>,
	D: DB + Clone,
> where
	Transaction<S, P, PureGeneratorPedersen, D>: Tagged,
{
	pub txs: Vec<TransactionWithContext<S, P, D>>,
}

#[derive(Debug, Clone)]
pub struct DeserializedTransactionsWithContext<S: SignatureKind<D>, P: ProofKind<D>, D: DB + Clone>
where
	Transaction<S, P, PureGeneratorPedersen, D>: Tagged,
{
	pub initial_tx: TransactionWithContext<S, P, D>,
	pub batches: Vec<DeserializedTransactionsWithContextBatch<S, P, D>>,
}

impl<S: SignatureKind<D>, P: ProofKind<D> + Send + Sync + 'static, D: DB + Clone>
	DeserializedTransactionsWithContext<S, P, D>
where
	<P as ProofKind<D>>::Pedersen: Send + Sync,
	Transaction<S, P, PureGeneratorPedersen, D>: Tagged,
{
	pub fn flat(self) -> Vec<TransactionWithContext<S, P, D>> {
		let mut result =
			Vec::with_capacity(1 + self.batches.iter().map(|b| b.txs.len()).sum::<usize>());
		result.push(self.initial_tx); // Add initial_tx first
		for batch in self.batches {
			result.extend(batch.txs); // Append each batch's transactions
		}
		result
	}

	pub fn network(&self) -> &str {
		self.initial_tx
			.tx
			.network_id()
			.or_else(|| {
				self.batches.iter().find_map(|b| b.txs.iter().find_map(|t| t.tx.network_id()))
			})
			.expect("no transaction in this batch had a network")
	}
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SerializedTransactionsWithContextBatch {
	pub txs: Vec<String>,
}

impl SerializedTransactionsWithContextBatch {
	fn new<S: SignatureKind<D>, P: ProofKind<D> + Send + Sync + 'static, D: DB + Clone>(
		batch_txs: &[TransactionWithContext<S, P, D>],
	) -> Result<Self, Box<dyn std::error::Error + Send + Sync>>
	where
		<P as ProofKind<D>>::Pedersen: Send + Sync,
		Transaction<S, P, PureGeneratorPedersen, D>: Tagged,
	{
		let txs = batch_txs
			.iter()
			.map(|tx_with_context| {
				// Serialize TransactionWithContext to a string
				serde_json::to_string(&tx_with_context).map_err(|e| Box::new(e))
			})
			.collect::<Result<Vec<String>, Box<_>>>()?;

		Ok(SerializedTransactionsWithContextBatch { txs })
	}
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SerializedTransactionsWithContext {
	pub initial_tx: String,
	pub batches: Vec<SerializedTransactionsWithContextBatch>,
}

impl SerializedTransactionsWithContext {
	pub fn new<S: SignatureKind<D>, P: ProofKind<D> + Send + Sync + 'static, D: DB + Clone>(
		txs: &DeserializedTransactionsWithContext<S, P, D>,
	) -> Result<Self, Box<dyn std::error::Error + Send + Sync>>
	where
		<P as ProofKind<D>>::Pedersen: Send + Sync,
		Transaction<S, P, PureGeneratorPedersen, D>: Tagged,
	{
		// Serialize initial_tx
		let initial_tx = serde_json::to_string(&txs.initial_tx).map_err(|e| Box::new(e))?;

		// Serialize each batch
		let batches = txs
			.batches
			.iter()
			.map(|batch| SerializedTransactionsWithContextBatch::new(&batch.txs))
			.collect::<Result<Vec<_>, Box<_>>>()?;

		Ok(SerializedTransactionsWithContext { initial_tx, batches })
	}
}
