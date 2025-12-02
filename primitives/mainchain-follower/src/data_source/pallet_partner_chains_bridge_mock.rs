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

use sidechain_domain::{McBlockHash, McBlockNumber};
use sp_partner_chains_bridge::{
	BridgeDataCheckpoint, BridgeTransferV1, MainChainScripts, TokenBridgeDataSource,
};
use std::io;

pub struct MidnightTokenBridgeDataSourceMock;

impl Default for MidnightTokenBridgeDataSourceMock {
	fn default() -> Self {
		Self::new()
	}
}

impl MidnightTokenBridgeDataSourceMock {
	pub fn new() -> Self {
		Self
	}
}

#[async_trait::async_trait]
impl<RecipientAddress: TryFrom<Vec<u8>> + Send + Sync> TokenBridgeDataSource<RecipientAddress>
	for MidnightTokenBridgeDataSourceMock
{
	async fn get_transfers(
		&self,
		_main_chain_scripts: MainChainScripts,
		data_checkpoint: BridgeDataCheckpoint,
		max_transfers: u32,
		_current_mc_block: McBlockHash,
	) -> Result<
		(Vec<BridgeTransferV1<RecipientAddress>>, BridgeDataCheckpoint),
		Box<dyn std::error::Error + Send + Sync>,
	> {
		// Keep the checkpoint unchanged if we are not allowed to return anything.
		if max_transfers == 0 {
			return Ok((Vec::new(), data_checkpoint));
		}

		// Progress one Cardano block per call using the last processed checkpoint as the anchor.
		let next_block = match data_checkpoint {
			BridgeDataCheckpoint::Block(block) => block.saturating_add(1_u32),
			// If the checkpoint is at a specific UTXO, start emitting from the beginning.
			BridgeDataCheckpoint::Utxo(_) => McBlockNumber(0),
		};

		let mut transfers = Vec::new();

		// Emit a couple of user transfers roughly every 4 blocks.
		if next_block.0 % 4 == 0 {
			let base_amount = 1_000 + u64::from(next_block.0);
			let count = usize::min(max_transfers as usize, 2);
			let recipient_bytes = {
				let mut bytes = [0u8; 32];
				bytes[..4].copy_from_slice(&next_block.0.to_le_bytes());
				bytes.to_vec()
			};

			for offset in 0..count {
				let recipient = RecipientAddress::try_from(recipient_bytes.clone()).map_err(|_| {
					io::Error::new(io::ErrorKind::Other, "mock recipient conversion failed")
				})?;

				transfers.push(BridgeTransferV1::UserTransfer {
					token_amount: base_amount + offset as u64,
					recipient,
				});
			}
		}

		Ok((transfers, BridgeDataCheckpoint::Block(next_block)))
	}
}
