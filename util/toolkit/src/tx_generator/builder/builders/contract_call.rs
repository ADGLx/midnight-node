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

use crate::{
	builder::{
		BuildContractAction, BuildInput, BuildIntent, BuildOutput, BuildTxs, BuildTxsExt, CallInfo,
		DeserializedTransactionsWithContext, IntentInfo, IntentToFile, MerkleTreeContract,
		OfferInfo, ProofProvider, ProofType, SignatureType, TransactionWithContext, Wallet,
		WalletSeed,
	},
	serde_def::SourceTransactions,
	tx_generator::builder::{ContractCallArgs, CreateIntentInfo},
};
use async_trait::async_trait;
use midnight_node_ledger_helpers::{ContractAddress, DB, ProofKind, SignatureKind};
use std::{convert::Infallible, marker::PhantomData, sync::Arc};

const CONTRACT_INPUT: u32 = 12;

pub struct ContractCallBuilder {
	call_key: &'static str,
	funding_seed: String,
	contract_address: ContractAddress,
	rng_seed: Option<[u8; 32]>,
}

impl ContractCallBuilder {
	pub fn new(args: ContractCallArgs) -> Self {
		let call_key: &'static str = Box::leak(args.call_key.into_boxed_str());

		Self {
			call_key,
			funding_seed: args.funding_seed,
			contract_address: args.contract_address,
			rng_seed: args.rng_seed,
		}
	}
}

#[async_trait]
impl<S: SignatureKind<D>, P: ProofKind<D> + std::fmt::Debug, D: DB + Clone> IntentToFile<S, P, D>
	for ContractCallBuilder
{
}

impl<S: SignatureKind<D>, P: ProofKind<D> + std::fmt::Debug, D: DB + Clone> BuildTxsExt<S, P, D>
	for ContractCallBuilder
{
	fn funding_seed(&self) -> WalletSeed {
		Wallet::<D>::wallet_seed_decode(&self.funding_seed)
	}

	fn rng_seed(&self) -> Option<[u8; 32]> {
		self.rng_seed
	}
}

impl<D: DB + Clone> CreateIntentInfo<D> for ContractCallBuilder {
	fn create_intent_info(&self) -> Box<dyn BuildIntent<D>> {
		println!("Create intent info for contract call");

		// - Contract Calls
		let call_contract: Box<dyn BuildContractAction<D>> = Box::new(CallInfo {
			type_: MerkleTreeContract::new(),
			address: self.contract_address,
			key: self.call_key,
			input: Box::new(CONTRACT_INPUT),
			_marker: PhantomData,
		});

		let actions: Vec<Box<dyn BuildContractAction<D>>> = vec![call_contract];

		// - Intents
		let intent_info = IntentInfo {
			guaranteed_unshielded_offer: None,
			fallible_unshielded_offer: None,
			actions,
		};

		Box::new(intent_info)
	}
}

#[async_trait]
impl<S: SignatureKind<D>, P: ProofKind<D> + std::fmt::Debug, D: DB + Clone> BuildTxs<S, P, D>
	for ContractCallBuilder
{
	type Error = Infallible;

	async fn build_txs_from(
		&self,
		received_tx: SourceTransactions<S, P, D>,
		prover_arc: Arc<dyn ProofProvider<D>>,
	) -> Result<DeserializedTransactionsWithContext<S, P, D>, Self::Error> {
		// - LedgerContext and TransactionInfo
		let (_, mut tx_info) = self.context_and_tx_info(received_tx, prover_arc);

		// - Intents
		let intent_info = self.create_intent_info();
		tx_info.add_intent(1, intent_info);

		//   - Input
		let inputs_info: Vec<Box<dyn BuildInput<D>>> = vec![];

		//   - Output
		let outputs_info: Vec<Box<dyn BuildOutput<D>>> = vec![];

		let offer_info =
			OfferInfo { inputs: inputs_info, outputs: outputs_info, transients: vec![] };

		tx_info.set_guaranteed_offer(offer_info);

		tx_info.set_funding_seeds(vec![<Self as BuildTxsExt<S, P, D>>::funding_seed(self)]);
		tx_info.use_mock_proofs_for_fees(false);

		#[cfg(not(feature = "erase-proof"))]
		let tx = tx_info.prove().await.expect("Balancing TX failed");

		#[cfg(feature = "erase-proof")]
		let tx = tx_info.erase_proof().await.expect("Balancing TX failed");

		let tx_with_context = TransactionWithContext::new(tx, None);

		Ok(DeserializedTransactionsWithContext { initial_tx: tx_with_context, batches: vec![] })
	}
}
