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

use async_trait::async_trait;
use midnight_node_ledger_helpers::{DB, ProofKind, SignatureKind};
use std::{convert::Infallible, sync::Arc};

use crate::{
	builder::{
		BuildTxs, ClaimMintInfo, DeserializedTransactionsWithContext, FromContext, LedgerContext,
		ProofProvider, ProofType, RewardsInfo, SignatureType, TransactionWithContext, Wallet,
	},
	serde_def::SourceTransactions,
	tx_generator::builder::ClaimRewardsArgs,
};

pub struct ClaimRewardsBuilder {
	funding_seed: String,
	rng_seed: Option<[u8; 32]>,
	amount: u128,
}

impl ClaimRewardsBuilder {
	pub fn new(args: ClaimRewardsArgs) -> Self {
		Self { funding_seed: args.funding_seed, rng_seed: args.rng_seed, amount: args.amount }
	}
}

#[async_trait]
impl<S: SignatureKind<D>, P: ProofKind<D> + std::fmt::Debug, D: DB + Clone> BuildTxs<S, P, D>
	for ClaimRewardsBuilder
{
	type Error = Infallible;
	async fn build_txs_from(
		&self,
		received_tx: SourceTransactions<S, P, D>,
		prover_arc: Arc<dyn ProofProvider<D>>,
	) -> Result<DeserializedTransactionsWithContext<S, P, D>, Self::Error> {
		// - Calculate the funding `WalletSeed` (can be more than one)
		let funding_seed = Wallet::<D>::wallet_seed_decode(&self.funding_seed);
		let inputs_wallet_seeds = vec![funding_seed];

		// initialize `LedgerContext` with the wallets
		let network_id = received_tx.network();
		let context = LedgerContext::new_from_wallet_seeds(network_id, &inputs_wallet_seeds);

		// update the context applying all existing previous txs queried from source (either genesis or live network)
		for block in received_tx.blocks {
			context.update_from_block(block.transactions, block.context, block.state_root.clone());
		}

		let context_arc = Arc::new(context);

		// - Transaction info
		let mut tx_info =
			ClaimMintInfo::new_from_context(context_arc.clone(), prover_arc.clone(), self.rng_seed);

		// - Mint
		let rewards = RewardsInfo { owner: funding_seed, value: self.amount };

		tx_info.set_rewards(rewards);

		#[cfg(not(feature = "erase-proof"))]
		let tx = tx_info.prove().await;

		#[cfg(feature = "erase-proof")]
		let tx = tx_info.erase_proof().await;

		let tx_with_context = TransactionWithContext::new(tx, None);

		Ok(DeserializedTransactionsWithContext { initial_tx: tx_with_context, batches: vec![] })
	}
}
