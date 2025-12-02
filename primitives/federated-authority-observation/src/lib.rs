//! # Federated Authority Observation Primitives
//!
//! This module provides primitives for observing federated authority changes from the main chain.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode};
use scale_info::TypeInfo;
use sidechain_domain::McBlockHash;
use sidechain_domain::{MainchainAddress, PolicyId};
use sp_api::decl_runtime_apis;
use sp_inherents::InherentIdentifier;
use sp_runtime::Vec;

#[cfg(feature = "std")]
use std::borrow::Cow;

use serde::{Deserialize, Serialize};

#[cfg(feature = "std")]
use serde::Deserializer;

#[cfg(feature = "std")]
use sp_core::{ByteArray, sr25519};

/// The inherent identifier for federated authority observation
pub const INHERENT_IDENTIFIER: InherentIdentifier = *b"faobsrve";

/// Alias for mainchain member identifier (28 bytes PolicyId)
pub type MainchainMember = PolicyId;

/// Convert Ed25519 public key to MainchainMember by taking first 28 bytes
#[cfg(feature = "std")]
pub fn ed25519_to_mainchain_member(public: sp_core::ed25519::Public) -> MainchainMember {
	let bytes = public.0;
	let mut mainchain_bytes = [0u8; 28];
	mainchain_bytes.copy_from_slice(&bytes[..28]);
	PolicyId(mainchain_bytes)
}

/// Custom deserializer for vector of hex-encoded sr25519 public keys
#[cfg(feature = "std")]
fn vec_hex_to_vec_sr25519<'de, D>(
	deserializer: D,
) -> Result<alloc::vec::Vec<sp_core::sr25519::Public>, D::Error>
where
	D: Deserializer<'de>,
{
	let strings: alloc::vec::Vec<alloc::string::String> =
		alloc::vec::Vec::deserialize(deserializer)?;
	strings
		.into_iter()
		.map(|s| {
			let s = s.strip_prefix("0x").ok_or_else(|| {
				serde::de::Error::custom(
					"sr25519 hex public key expected to be prepended with `0x`",
				)
			})?;
			let bytes = hex::decode(s).map_err(serde::de::Error::custom)?;
			sr25519::Public::from_slice(&bytes)
				.map_err(|_| serde::de::Error::custom("Invalid sr25519 public key length"))
		})
		.collect()
}

#[derive(Eq, Debug, Clone, PartialEq, TypeInfo, Default, Encode, Decode, PartialOrd, Ord)]
pub struct AuthorityMemberPublicKey(pub Vec<u8>);

/// Struct containing all mainchain script information for a governance body
#[derive(
	Debug,
	Clone,
	PartialEq,
	Eq,
	Encode,
	Decode,
	DecodeWithMemTracking,
	TypeInfo,
	Default,
	Serialize,
	Deserialize,
)]
pub struct MainChainScripts {
	/// The script address for managing members on Cardano
	pub address: MainchainAddress,
	/// The policy ID for the governance body's native asset
	pub policy_id: PolicyId,
	/// The governance contract address for two-stage upgrades
	pub governance_address: MainchainAddress,
	/// The governance policy ID (NFT) for two-stage upgrades
	pub governance_policy_id: PolicyId,
}

/// Struct containing round information for contract upgrades
#[derive(
	Debug,
	Clone,
	PartialEq,
	Eq,
	Encode,
	Decode,
	DecodeWithMemTracking,
	TypeInfo,
	Default,
	Serialize,
	Deserialize,
)]
pub struct RoundInfo {
	/// The previous contract upgrade round
	pub previous_round: u8,
	/// The current contract upgrade round
	pub current_round: u8,
	/// The next expected contract upgrade round
	pub next_round: u8,
}

/// Placeholder structure for federated authority data from main chain
/// This will contain sr25519 public keys and mainchain member hashes for federated authorities
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, TypeInfo)]
pub struct FederatedAuthorityData {
	/// Council contract upgrade round value observed from the governance contract
	pub council_round: Option<u8>,
	/// Technical Committee contract upgrade round value observed from the governance contract
	pub technical_committee_round: Option<u8>,
	/// List of tuples (sr25519 authority public key, mainchain member hash)
	pub council_authorities: Vec<(AuthorityMemberPublicKey, MainchainMember)>,
	/// List of tuples (sr25519 authority public key, mainchain member hash)
	pub technical_committee_authorities: Vec<(AuthorityMemberPublicKey, MainchainMember)>,
	/// Main chain block hash this data was observed at
	pub mc_block_hash: McBlockHash,
}

/// Error type for federated authority observation inherents
#[derive(Encode, Debug)]
#[cfg_attr(feature = "std", derive(Decode))]
pub enum InherentError {
	/// The inherent data could not be decoded
	DecodeFailed,
	/// Council round value is less than current round
	InvalidCouncilRound,
	/// Technical Committee round value is less than current round
	InvalidTechnicalCommitteeRound,
	/// Other error
	#[cfg(feature = "std")]
	Other(Cow<'static, str>),
}

impl sp_inherents::IsFatalError for InherentError {
	fn is_fatal_error(&self) -> bool {
		true
	}
}

/// Custom deserializer for vector of hex-encoded mainchain member hashes (MainchainMember)
#[cfg(feature = "std")]
fn vec_hex_to_vec_mainchain_member<'de, D>(
	deserializer: D,
) -> Result<alloc::vec::Vec<MainchainMember>, D::Error>
where
	D: Deserializer<'de>,
{
	let strings: alloc::vec::Vec<alloc::string::String> =
		alloc::vec::Vec::deserialize(deserializer)?;
	strings
		.into_iter()
		.map(|s| MainchainMember::decode_hex(&s).map_err(serde::de::Error::custom))
		.collect()
}

/// Configuration for observing a governance body
#[cfg(feature = "std")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthBodyConfig {
	/// The Cardano script address for this governance body
	pub address: String,
	/// The policy ID for the native asset associated with this governance body
	pub policy_id: PolicyId,
	/// Initial members of this governance body (for genesis)
	#[serde(deserialize_with = "vec_hex_to_vec_sr25519")]
	pub members: Vec<sp_core::sr25519::Public>,
	/// Initial mainchain member hashes (for genesis)
	#[serde(deserialize_with = "vec_hex_to_vec_mainchain_member")]
	pub members_mainchain: Vec<MainchainMember>,
	/// The Cardano script address for the governance contract (for two-stage upgrades)
	pub governance_address: String,
	/// The policy ID for the governance contract NFT (for two-stage upgrades)
	pub governance_policy_id: PolicyId,
}

/// Configuration for Federated Authority Observation
#[cfg(feature = "std")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FederatedAuthorityObservationConfig {
	/// Council governance body configuration
	pub council: AuthBodyConfig,
	/// Technical Committee governance body configuration
	pub technical_committee: AuthBodyConfig,
}

decl_runtime_apis! {
	pub trait FederatedAuthorityObservationApi {
		/// Get the Council mainchain scripts (address, policy_id, governance_address, governance_policy_id)
		fn get_council_scripts() -> MainChainScripts;
		/// Get the Technical Committee mainchain scripts
		fn get_technical_committee_scripts() -> MainChainScripts;
		/// Get Council round information (previous, current, next)
		fn get_council_round_info() -> RoundInfo;
		/// Get Technical Committee round information (previous, current, next)
		fn get_technical_committee_round_info() -> RoundInfo;
	}
}
