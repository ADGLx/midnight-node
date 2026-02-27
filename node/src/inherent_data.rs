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
use authority_selection_inherents::CommitteeMember;
use authority_selection_inherents::{
	AriadneInherentDataProvider as AriadneIDP, AuthoritySelectionDataSource,
	AuthoritySelectionInputs,
};
use derive_new::new;
use midnight_node_runtime::{
	CrossChainPublic,
	opaque::{Block, SessionKeys},
};
use midnight_primitives::BridgeRecipient;
use midnight_primitives_cnight_observation::CNightObservationApi;
use midnight_primitives_federated_authority_observation::FederatedAuthorityObservationApi;
use sc_consensus_aura::{SlotDuration, find_pre_digest};
use sc_service::Arc;
use sidechain_domain::{McBlockHash, ScEpochNumber, mainchain_epoch::MainchainEpochConfig};
use sidechain_mc_hash::McHashDataSource;
use sidechain_mc_hash::McHashInherentDataProvider as McHashIDP;
use sidechain_slots::ScSlotConfig;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_consensus_aura::{Slot, sr25519::AuthorityPair as AuraPair};
use sp_core::Pair;
use sp_inherents::CreateInherentDataProviders;
use sp_partner_chains_bridge::{
	TokenBridgeDataSource, TokenBridgeIDPRuntimeApi, TokenBridgeInherentDataProvider,
};
use sp_partner_chains_consensus_aura::CurrentSlotProvider;
use sp_runtime::traits::{Block as BlockT, Header, Zero};
use sp_session_validator_management::SessionValidatorManagementApi;
use sp_timestamp::Timestamp;
use std::error::Error;
use time_source::TimeSource;

use midnight_primitives_mainchain_follower::{
	FederatedAuthorityObservationDataSource, MidnightCNightObservationDataSource,
	idp::{FederatedAuthorityInherentDataProvider, MidnightCNightObservationInherentDataProvider},
};

//#[cfg(feature = "experimental")]
//use {midnight_node_runtime::BeneficiaryId, sp_block_rewards::BlockBeneficiaryInherentProvider};
#[allow(clippy::too_many_arguments)]
#[derive(new)]
pub(crate) struct ProposalCIDP<T> {
	config: CreateInherentDataConfig,
	client: Arc<T>,
	mc_hash_data_source: Arc<dyn McHashDataSource + Send + Sync>,
	authority_selection_data_source: Arc<dyn AuthoritySelectionDataSource + Send + Sync>,
	cnight_observation_data_source: Arc<dyn MidnightCNightObservationDataSource + Send + Sync>,
	federated_authority_observation_data_source:
		Arc<dyn FederatedAuthorityObservationDataSource + Send + Sync>,
	bridge_data_source: Arc<dyn TokenBridgeDataSource<BridgeRecipient> + Send + Sync>,
}

#[async_trait]
impl<T> CreateInherentDataProviders<Block, ()> for ProposalCIDP<T>
where
	T: ProvideRuntimeApi<Block> + Send + Sync + 'static,
	T: HeaderBackend<Block>,
	T::Api: SessionValidatorManagementApi<
			Block,
			CommitteeMember<CrossChainPublic, SessionKeys>,
			AuthoritySelectionInputs,
			ScEpochNumber,
		>,
	T::Api: CNightObservationApi<Block>,
	T::Api: FederatedAuthorityObservationApi<Block>,
	T::Api: TokenBridgeIDPRuntimeApi<Block>,
{
	type InherentDataProviders = (
		sp_consensus_aura::inherents::InherentDataProvider,
		sp_timestamp::InherentDataProvider,
		McHashIDP,
		AriadneIDP,
		//BlockBeneficiaryInherentProvider<BeneficiaryId>,
		MidnightCNightObservationInherentDataProvider,
		FederatedAuthorityInherentDataProvider,
		TokenBridgeInherentDataProvider<BridgeRecipient>,
	);

	async fn create_inherent_data_providers(
		&self,
		parent_hash: <Block as BlockT>::Hash,
		_extra_args: (),
	) -> Result<Self::InherentDataProviders, Box<dyn std::error::Error + Send + Sync>> {
		let Self {
			config,
			client,
			mc_hash_data_source,
			authority_selection_data_source,
			cnight_observation_data_source,
			federated_authority_observation_data_source,
			bridge_data_source,
		} = self;

		let CreateInherentDataConfig { mc_epoch_config, sc_slot_config, time_source } = config;

		let (slot, timestamp) =
			timestamp_and_slot_cidp(sc_slot_config.slot_duration, time_source.clone());

		let parent_header = client
			.header(parent_hash)?
			.ok_or_else(|| format!("Missing parent header for {parent_hash:?}"))?;

		let mc_hash = McHashIDP::new_proposal(
			parent_header,
			mc_hash_data_source.as_ref(),
			*slot,
			sc_slot_config.slot_duration,
		)
		.await?;

		let ariadne_data_provider = AriadneIDP::new(
			client.as_ref(),
			sc_slot_config,
			mc_epoch_config,
			parent_hash,
			*slot,
			authority_selection_data_source.as_ref(),
			mc_hash.mc_epoch(),
		)
		.await?;
		/*
		#[cfg(feature = "experimental")]
		let block_beneficiary_provider = BlockBeneficiaryInherentProvider::<BeneficiaryId>::from_env(
			"SIDECHAIN_BLOCK_BENEFICIARY",
		)?;
		 */

		let cnight_observation = MidnightCNightObservationInherentDataProvider::new(
			client.clone(),
			cnight_observation_data_source.as_ref(),
			parent_hash,
			mc_hash.mc_hash(),
		)
		.await?;

		let federated_authority = FederatedAuthorityInherentDataProvider::new(
			client.clone(),
			federated_authority_observation_data_source.as_ref(),
			parent_hash,
			&mc_hash.mc_hash(),
		)
		.await?;

		let bridge = TokenBridgeInherentDataProvider::new(
			client.as_ref(),
			parent_hash,
			mc_hash.mc_hash(),
			bridge_data_source.as_ref(),
		)
		.await?;

		Ok((
			slot,
			timestamp,
			mc_hash,
			ariadne_data_provider,
			//#[cfg(feature = "experimental")]
			//block_beneficiary_provider,
			cnight_observation,
			federated_authority,
			bridge,
		))
	}
}

#[allow(clippy::too_many_arguments)]
#[derive(new)]
pub struct VerifierCIDP<T> {
	config: CreateInherentDataConfig,
	client: Arc<T>,
	mc_hash_data_source: Arc<dyn McHashDataSource + Send + Sync>,
	authority_selection_data_source: Arc<dyn AuthoritySelectionDataSource + Send + Sync>,
	cnight_observation_data_source: Arc<dyn MidnightCNightObservationDataSource + Send + Sync>,
	federated_authority_observation_data_source:
		Arc<dyn FederatedAuthorityObservationDataSource + Send + Sync>,
	bridge_data_source: Arc<dyn TokenBridgeDataSource<BridgeRecipient> + Send + Sync>,
}

impl<T: Send + Sync> CurrentSlotProvider for VerifierCIDP<T> {
	fn slot(&self) -> Slot {
		*timestamp_and_slot_cidp(self.config.slot_duration(), self.config.time_source.clone()).0
	}
}

#[async_trait]
impl<T> CreateInherentDataProviders<Block, (Slot, McBlockHash)> for VerifierCIDP<T>
where
	T: ProvideRuntimeApi<Block> + Send + Sync + HeaderBackend<Block> + 'static,
	T::Api: SessionValidatorManagementApi<
			Block,
			CommitteeMember<CrossChainPublic, SessionKeys>,
			AuthoritySelectionInputs,
			ScEpochNumber,
		>,
	T::Api: CNightObservationApi<Block>,
	T::Api: FederatedAuthorityObservationApi<Block>,
	T::Api: TokenBridgeIDPRuntimeApi<Block>,
{
	type InherentDataProviders = (
		sp_timestamp::InherentDataProvider,
		AriadneIDP,
		MidnightCNightObservationInherentDataProvider,
		FederatedAuthorityInherentDataProvider,
		TokenBridgeInherentDataProvider<BridgeRecipient>,
	);

	async fn create_inherent_data_providers(
		&self,
		parent_hash: <Block as BlockT>::Hash,
		(verified_block_slot, mc_hash): (Slot, McBlockHash),
	) -> Result<Self::InherentDataProviders, Box<dyn Error + Send + Sync>> {
		let Self {
			config,
			client,
			mc_hash_data_source,
			authority_selection_data_source,
			cnight_observation_data_source,
			federated_authority_observation_data_source,
			bridge_data_source,
		} = self;

		let CreateInherentDataConfig { mc_epoch_config, sc_slot_config, time_source, .. } = config;

		let timestamp = sp_timestamp::InherentDataProvider::new(Timestamp::new(
			time_source.get_current_time_millis(),
		));
		let parent_header = client.expect_header(parent_hash)?;
		let parent_slot = slot_from_predigest(&parent_header)?;
		let mc_state_reference = McHashIDP::new_verification(
			parent_header,
			parent_slot,
			verified_block_slot,
			mc_hash.clone(),
			config.slot_duration(),
			mc_hash_data_source.as_ref(),
		)
		.await?;

		let ariadne_data_provider = AriadneIDP::new(
			client.as_ref(),
			sc_slot_config,
			mc_epoch_config,
			parent_hash,
			verified_block_slot,
			authority_selection_data_source.as_ref(),
			mc_state_reference.epoch,
		)
		.await?;

		let cnight_observation = MidnightCNightObservationInherentDataProvider::new(
			client.clone(),
			cnight_observation_data_source.as_ref(),
			parent_hash,
			mc_hash.clone(),
		)
		.await?;

		let federated_authority = FederatedAuthorityInherentDataProvider::new(
			client.clone(),
			federated_authority_observation_data_source.as_ref(),
			parent_hash,
			&mc_hash,
		)
		.await?;

		let bridge = TokenBridgeInherentDataProvider::new(
			client.as_ref(),
			parent_hash,
			mc_hash,
			bridge_data_source.as_ref(),
		)
		.await?;

		Ok((timestamp, ariadne_data_provider, cnight_observation, federated_authority, bridge))
	}
}

pub fn slot_from_predigest(
	header: &<Block as BlockT>::Header,
) -> Result<Option<Slot>, Box<dyn Error + Send + Sync>> {
	if header.number().is_zero() {
		// genesis block doesn't have a slot
		Ok(None)
	} else {
		Ok(Some(find_pre_digest::<Block, <AuraPair as Pair>::Signature>(header)?))
	}
}

#[derive(Debug, thiserror::Error)]
pub enum InherentDataConfigError {
	#[error(
		"Mainchain epoch duration ({epoch_duration_millis}ms) must be divisible \
		 by mainchain slot duration ({slot_duration_millis}ms)"
	)]
	MainchainEpochNotDivisible { epoch_duration_millis: u64, slot_duration_millis: u64 },
}

#[derive(Clone)]
pub(crate) struct CreateInherentDataConfig {
	pub mc_epoch_config: MainchainEpochConfig,
	pub sc_slot_config: ScSlotConfig,
	pub time_source: Arc<dyn TimeSource + Send + Sync + 'static>,
}

impl CreateInherentDataConfig {
	pub fn new(
		mc_epoch_config: MainchainEpochConfig,
		sc_slot_config: ScSlotConfig,
		time_source: Arc<dyn TimeSource + Send + Sync + 'static>,
	) -> Result<Self, InherentDataConfigError> {
		let mc_epoch_dur = mc_epoch_config.epoch_duration_millis.millis();
		let mc_slot_dur = mc_epoch_config.slot_duration_millis.millis();

		if mc_slot_dur > 0 && !mc_epoch_dur.is_multiple_of(mc_slot_dur) {
			return Err(InherentDataConfigError::MainchainEpochNotDivisible {
				epoch_duration_millis: mc_epoch_dur,
				slot_duration_millis: mc_slot_dur,
			});
		}

		Ok(Self { mc_epoch_config, sc_slot_config, time_source })
	}

	pub fn slot_duration(&self) -> SlotDuration {
		self.sc_slot_config.slot_duration
	}
}

fn timestamp_and_slot_cidp(
	slot_duration: SlotDuration,
	time_source: Arc<dyn TimeSource + Send + Sync>,
) -> (sp_consensus_aura::inherents::InherentDataProvider, sp_timestamp::InherentDataProvider) {
	let timestamp = sp_timestamp::InherentDataProvider::new(Timestamp::new(
		time_source.get_current_time_millis(),
	));
	let slot = sp_consensus_aura::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
		*timestamp,
		slot_duration,
	);
	(slot, timestamp)
}

#[cfg(test)]
mod tests {
	use super::*;
	use sc_consensus_aura::SlotDuration;
	use sidechain_domain::mainchain_epoch::{Duration, MainchainEpochConfig, Timestamp};
	use sidechain_slots::{ScSlotConfig, SlotsPerEpoch};
	use time_source::SystemTimeSource;

	fn valid_mc_epoch_config() -> MainchainEpochConfig {
		use midnight_primitives::cardano_mainnet::*;
		MainchainEpochConfig {
			epoch_duration_millis: Duration::from_millis(MC_EPOCH_DURATION_MILLIS),
			slot_duration_millis: Duration::from_millis(MC_SLOT_DURATION_MILLIS),
			first_epoch_timestamp_millis: Timestamp::from_unix_millis(
				MC_FIRST_EPOCH_TIMESTAMP_MILLIS,
			),
			first_epoch_number: MC_FIRST_EPOCH_NUMBER,
			first_slot_number: MC_FIRST_SLOT_NUMBER,
		}
	}

	fn valid_sc_slot_config() -> ScSlotConfig {
		ScSlotConfig {
			slots_per_epoch: SlotsPerEpoch(300),
			slot_duration: SlotDuration::from_millis(6_000),
		}
	}

	fn time_source() -> Arc<dyn TimeSource + Send + Sync + 'static> {
		Arc::new(SystemTimeSource)
	}

	#[test]
	fn new_succeeds_with_valid_config() {
		let result = CreateInherentDataConfig::new(
			valid_mc_epoch_config(),
			valid_sc_slot_config(),
			time_source(),
		);
		assert!(result.is_ok());
		let config = result.unwrap();
		assert_eq!(config.mc_epoch_config, valid_mc_epoch_config());
		assert_eq!(config.slot_duration().as_millis(), 6_000);
	}

	#[test]
	fn new_rejects_mc_epoch_not_divisible_by_mc_slot() {
		let mut mc = valid_mc_epoch_config();
		mc.epoch_duration_millis = Duration::from_millis(1_000_000);
		mc.slot_duration_millis = Duration::from_millis(3_000);
		let result = CreateInherentDataConfig::new(mc, valid_sc_slot_config(), time_source());
		assert!(matches!(
			result,
			Err(InherentDataConfigError::MainchainEpochNotDivisible {
				epoch_duration_millis: 1_000_000,
				slot_duration_millis: 3_000,
			})
		));
	}
}
