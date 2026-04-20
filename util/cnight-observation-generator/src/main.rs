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

//! Offline generator for the cNIGHT observation snapshot file.
//!
//! Runs the four observation queries (registrations, deregistrations, asset
//! creates, asset spends) across the entire cNIGHT observation range and
//! writes the result as a scale-encoded file, suitable for loading by the
//! node via `CNIGHT_OBSERVATION_FILE`.

use clap::Parser;
use midnight_primitives_cnight_observation::CNightAddresses;
use midnight_primitives_mainchain_follower::data_source::{
	CNightObservationSnapshot, SnapshotInputs, get_connection,
};
use std::path::PathBuf;
use std::time::{Duration, Instant};

#[derive(Parser, Debug)]
#[command(about = "Generate a cNIGHT observation snapshot file from db-sync")]
struct Args {
	/// postgres connection string to db-sync (e.g.
	/// `postgresql://user:pass@host:5432/cexplorer?sslmode=disable`).
	#[arg(long, env = "DB_SYNC_POSTGRES_CONNECTION_STRING")]
	postgres_url: String,

	/// Hex-encoded cNIGHT policy id (28 bytes / 56 hex chars).
	#[arg(long)]
	cnight_policy_id: String,

	/// cNIGHT asset name (typically "NIGHT").
	#[arg(long, default_value = "NIGHT")]
	cnight_asset_name: String,

	/// Mapping validator address (bech32, typically `addr1w…`).
	#[arg(long)]
	mapping_validator_address: String,

	/// Auth token asset name.
	#[arg(long, default_value = "")]
	auth_token_asset_name: String,

	/// End of observation range (Cardano block number). The snapshot will
	/// cover all cNIGHT activity in `[0, end_block_no]`.
	#[arg(long)]
	end_block_no: u32,

	/// Cardano network magic (mainnet=764824073, preprod=1, preview=2).
	/// Baked into the snapshot's inputs_hash so nodes refuse a wrong-network file.
	#[arg(long)]
	cardano_network_magic: u32,

	/// Output file path.
	#[arg(long)]
	output: PathBuf,
}

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
	let args = Args::parse();

	let policy_bytes = hex::decode(args.cnight_policy_id.trim_start_matches("0x"))?;
	let cnight_policy_id: [u8; 28] = policy_bytes
		.as_slice()
		.try_into()
		.map_err(|_| format!("cnight_policy_id must be 28 bytes, got {}", policy_bytes.len()))?;

	let pool = get_connection(&args.postgres_url, Duration::from_secs(30)).await?;
	println!("connected to postgres");

	let config = CNightAddresses {
		mapping_validator_address: args.mapping_validator_address,
		auth_token_asset_name: args.auth_token_asset_name,
		cnight_policy_id,
		cnight_asset_name: args.cnight_asset_name,
	};

	println!("generating snapshot (end_block_no = {})...", args.end_block_no);
	let t0 = Instant::now();
	let snapshot = CNightObservationSnapshot::generate(pool, &config, args.end_block_no).await?;
	let elapsed = t0.elapsed();

	println!(
		"snapshot built in {:.1}s: {} registrations, {} deregistrations, {} creates, {} spends ({} total events)",
		elapsed.as_secs_f32(),
		snapshot.registrations.len(),
		snapshot.deregistrations.len(),
		snapshot.creates.len(),
		snapshot.spends.len(),
		snapshot.total_events(),
	);

	let inputs_hash = SnapshotInputs {
		cardano_network_magic: args.cardano_network_magic,
		cnight_policy_id: &config.cnight_policy_id,
		cnight_asset_name: config.cnight_asset_name.as_bytes(),
		mapping_validator_address: &config.mapping_validator_address,
		auth_token_asset_name: &config.auth_token_asset_name,
	}
	.hash();

	snapshot.write_to_path(&args.output, inputs_hash)?;
	let size = std::fs::metadata(&args.output)?.len();
	println!(
		"wrote {} ({:.1} MB, inputs_hash={})",
		args.output.display(),
		size as f64 / 1_048_576.0,
		hex::encode(inputs_hash),
	);
	Ok(())
}
