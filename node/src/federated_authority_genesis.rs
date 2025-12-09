use std::{fs::File, path::Path, sync::Arc};

use midnight_primitives_federated_authority_observation::{
	AuthBodyConfig, AuthorityMemberPublicKey, FederatedAuthorityObservationConfig, MainchainMember,
};
use midnight_primitives_mainchain_follower::FederatedAuthorityObservationDataSource;
use sidechain_domain::{McBlockHash, PolicyId};

use sp_core::{ByteArray, sr25519::Public};

#[derive(Debug, thiserror::Error)]
pub enum FederatedAuthorityGenesisError {
	#[error("Failed to serialize UTXOs to JSON: {0}")]
	SerdeError(#[from] serde_json::Error),

	#[error("Failed retrieving from data source: {0}")]
	DatasourceError(String),
}

/// Saves as json file the Federated Authority Genesis Config
pub async fn generate_federated_authority_genesis(
	council_address: String,
	council_policy_id: PolicyId,
	technical_committee_address: String,
	technical_committee_policy_id: PolicyId,
	federated_authority_observation_data_source: Arc<dyn FederatedAuthorityObservationDataSource>,
	// Cardano block hash("mc hash") which is assumed to be the tip for the queries
	cardano_tip: McBlockHash,
	output_path: impl AsRef<Path>,
) -> Result<(), FederatedAuthorityGenesisError> {
	let council = AuthBodyConfig {
		address: council_address,
		policy_id: council_policy_id,
		members: vec![],
		members_mainchain: vec![],
	};

	let technical_committee = AuthBodyConfig {
		address: technical_committee_address,
		policy_id: technical_committee_policy_id,
		members: vec![],
		members_mainchain: vec![],
	};

	let mut config = FederatedAuthorityObservationConfig { council, technical_committee };

	// get the sr25519 public keys and mainchain members
	let mut data = federated_authority_observation_data_source
		.get_federated_authority_data(&config, &cardano_tip)
		.await
		.map_err(|e| FederatedAuthorityGenesisError::DatasourceError(e.to_string()))?;

	// update the members of the council
	let (council_members, council_mainchain_members) =
		get_members_and_mainchain_members(data.council_authorities.into_iter());
	config.council.members = council_members;
	config.council.members_mainchain = council_mainchain_members;

	// update the members of the technical committee
	let (technical_committee_members, technical_committee_mainchain_members) =
		get_members_and_mainchain_members(data.technical_committee_authorities.into_iter());
	config.technical_committee.members = technical_committee_members;
	config.technical_committee.members_mainchain = technical_committee_mainchain_members;

	let json = serde_json::to_string_pretty(&config)?;
	let mut file = File::create(output_path.as_ref()).await?;
	file.write_all(json.as_bytes()).await?;
	log::info!("Wrote Federated Authority genesis to {}", output_path.as_ref().display());

	Ok(())
}

// helper function to separate a list of tuples, into separate list of their own.
// e.g. list of (elem_1, elem_2) becomes ( list of elem_1's, list of elem_2's)
fn get_members_and_mainchain_members(
	iterator: std::vec::IntoIter<(AuthorityMemberPublicKey, MainchainMember)>,
) -> (Vec<Public>, Vec<MainchainMember>) {
	iterator.fold(
		(Vec::<Public>::new(), Vec::<MainchainMember>::new()),
		|(mut members, mut mainchain_members), (member, mainchain_member)| {
			match Public::from_slice(member.0.as_slice()) {
				Ok(member) => members.push(member),
				Err(_) => log::warn!("Failed to convert to s255519 key: {}", hex::encode(member.0)),
			};

			mainchain_members.push(mainchain_member);

			(members, mainchain_members)
		},
	)
}
