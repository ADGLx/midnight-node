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

use midnight_node_ledger_helpers::*;
use std::{path::Path, sync::Arc};
use thiserror::Error;

use crate::{
	remote_prover::RemoteProofServer,
	serde_def::{DeserializedTransactionsWithContext, SourceTransactions},
};

pub mod builder;
pub mod destination;
pub mod source;

use builder::{BuildTxs, Builder, DynamicError};
use destination::{Destination, SendTxs, SendTxsToFile, SendTxsToUrl};
use source::{GetTxs, GetTxsFromFile, GetTxsFromUrl, Source, SourceError};

#[derive(Debug, Error)]
pub enum TxGeneratorError {
	#[error("invalid source: {0}")]
	SourceError(#[from] SourceError),
	#[error("invalid destination: {0}")]
	DestinationError(#[from] DestinationError),
}

#[derive(Debug, Error)]
#[error("failed to create OnlineClient: {source}")]
pub struct DestinationError {
	#[from]
	source: subxt::Error,
}

pub struct TxGenerator<S: SignatureKind<D>, P: ProofKind<D> + Send + Sync + 'static, D: DB + Clone>
where
	Transaction<S, P, PedersenRandomness, D>: Tagged,
{
	pub source: Box<dyn GetTxs<S, P, D>>,
	pub destinations: Vec<Box<dyn SendTxs<S, P, D>>>,
	pub builder: Box<dyn BuildTxs<S, P, D, Error = DynamicError>>,
	pub prover: Arc<dyn ProofProvider<D>>,
}

impl<
	S: SignatureKind<D> + Tagged + Send + Sync + 'static,
	P: ProofKind<D> + Send + Sync + 'static + std::fmt::Debug,
	D: DB + Clone + 'static,
> TxGenerator<S, P, D>
where
	<P as ProofKind<D>>::Pedersen: Send + Sync,
	<P as ProofKind<D>>::LatestProof: Send + Sync,
	<P as ProofKind<D>>::Proof: Send + Sync,
	Transaction<S, P, PedersenRandomness, D>: Tagged,
{
	pub async fn new(
		src: Source,
		dest: Destination,
		builder: Builder,
		proof_server: Option<String>,
		dry_run: bool,
	) -> Result<Self, TxGeneratorError> {
		let source = Self::source(src, dry_run).await?;
		let destinations = Self::destinations(dest, dry_run).await?;
		let builder = builder.to_builder::<S, P, D>(dry_run);
		let prover = Self::prover(proof_server, dry_run);

		Ok(Self { source, destinations, builder, prover })
	}

	pub async fn source(
		src: Source,
		dry_run: bool,
	) -> Result<Box<dyn GetTxs<S, P, D>>, SourceError> {
		if let Some(ref src_files) = src.src_files {
			if dry_run {
				println!("Dry-run: Source transactions from file(s): {:?}", &src_files);
				return Ok(Box::new(()));
			}
			let path = Path::new(&src_files[0]);
			let extension = path.extension().and_then(|ext| ext.to_str()).unwrap_or("");
			let source: Box<dyn GetTxs<S, P, D>> = Box::new(GetTxsFromFile::new(
				src_files.clone(),
				extension.to_string(),
				src.dust_warp,
			));
			Ok(source)
		} else if let Some(url) = src.src_url {
			if dry_run {
				println!("Dry-run: Source transactions from url: {:?}", &url);
				return Ok(Box::new(()));
			}
			let source: Box<dyn GetTxs<S, P, D>> = Box::new(GetTxsFromUrl::new(
				&url,
				src.fetch_concurrency,
				src.dust_warp,
				src.fetch_cache,
			));
			Ok(source)
		} else {
			Err(SourceError::InvalidSourceArgs(src))
		}
	}

	async fn destinations(
		dest: Destination,
		dry_run: bool,
	) -> Result<Vec<Box<dyn SendTxs<S, P, D>>>, DestinationError> {
		if let Some(ref dest_file) = dest.dest_file {
			if dry_run {
				println!("Dry-run: Destination file: {:?}", &dest_file);
				if dest.to_bytes {
					println!("Dry-run: Destination file-format: bytes");
				} else {
					println!("Dry-run: Destination file-format: json");
				}
				return Ok(vec![Box::new(())]);
			}
			let destination: Box<dyn SendTxs<S, P, D>> =
				Box::new(SendTxsToFile::new(dest_file.clone(), dest.to_bytes));

			return Ok(vec![destination]);
		}

		// ------ accept multiple urls ------
		let mut dests = vec![];
		for url in dest.dest_urls {
			if dry_run {
				println!("Dry-run: Destination RPC: {:?}", &url);
				println!("Dry-run: Destination rate: {:?} TPS", &dest.rate);
				continue;
			}
			let destination: Box<dyn SendTxs<S, P, D>> =
				Box::new(SendTxsToUrl::<S, P, D>::new(url.clone(), dest.rate));

			dests.push(destination);
		}

		Ok(dests)
	}

	pub fn prover(proof_server: Option<String>, dry_run: bool) -> Arc<dyn ProofProvider<D>> {
		if let Some(url) = proof_server {
			if dry_run {
				println!("Dry-run: remove prover: {url}");
			}
			Arc::new(RemoteProofServer::new(url))
		} else {
			if dry_run {
				println!("Dry-run: local prover (no proof server)");
			}
			Arc::new(LocalProofServer::new())
		}
	}

	pub async fn get_txs(
		&self,
	) -> Result<SourceTransactions<S, P, D>, Box<dyn std::error::Error + Send + Sync>> {
		self.source.get_txs().await
	}

	pub async fn send_txs(
		&self,
		txs: &DeserializedTransactionsWithContext<S, P, D>,
	) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
		let sends_txs_futs: Vec<_> =
			self.destinations.iter().map(|dest| dest.send_txs(txs)).collect();

		// send transactions concurrently; no waiting needed for prev async calls
		let results = futures::future::join_all(sends_txs_futs).await;

		for result in results.iter() {
			if let Err(e) = result {
				println!("ERROR: {e}");
			}
		}

		Ok(())
	}
}

impl<S: SignatureKind<D>, P: ProofKind<D>, D: DB + Clone> TxGenerator<S, P, D>
where
	Transaction<S, P, PedersenRandomness, D>: Tagged,
{
	pub async fn build_txs(
		&self,
		received_txs: &SourceTransactions<S, P, D>,
	) -> Result<DeserializedTransactionsWithContext<S, P, D>, DynamicError> {
		self.builder.build_txs_from(received_txs.clone(), self.prover.clone()).await
	}
}
