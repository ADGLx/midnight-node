#![allow(dead_code)]

use sp_consensus_beefy::{
	Payload,
	mmr::{BeefyAuthoritySet, BeefyNextAuthoritySet},
};
use sp_core::H256;

use crate::Error;

pub type BeefyStakes = Vec<(sp_consensus_beefy::ecdsa_crypto::AuthorityId, u64)>;

pub const CURRENT_BEEFY_STAKES_ID: sp_consensus_beefy::BeefyPayloadId = *b"cs";
pub const CURRENT_BEEFY_AUTHORITY_SET: sp_consensus_beefy::BeefyPayloadId = *b"cb";
pub const NEXT_BEEFY_STAKES_ID: sp_consensus_beefy::BeefyPayloadId = *b"ns";
pub const NEXT_BEEFY_AUTHORITY_SET: sp_consensus_beefy::BeefyPayloadId = *b"nb";

#[derive(Debug)]
pub struct BeefyStakesInfo {
	current_stakes: BeefyStakes,
	current_authority_set: BeefyAuthoritySet<H256>,
	next_stakes: BeefyStakes,
	next_authority_set: BeefyNextAuthoritySet<H256>,
}

impl TryFrom<Payload> for BeefyStakesInfo {
	type Error = Error;

	fn try_from(value: Payload) -> Result<Self, Self::Error> {
		let current_stakes: BeefyStakes = value
			.get_decoded(&CURRENT_BEEFY_STAKES_ID)
			.ok_or(Error::MissingCurrentBeefyStakes)?;
		let current_authority_set: BeefyAuthoritySet<H256> = value
			.get_decoded(&CURRENT_BEEFY_AUTHORITY_SET)
			.ok_or(Error::MissingCurrentAuthoritySet)?;

		let next_stakes: BeefyStakes =
			value.get_decoded(&NEXT_BEEFY_STAKES_ID).ok_or(Error::MissingNextBeefyStakes)?;
		let next_authority_set: BeefyNextAuthoritySet<H256> = value
			.get_decoded(&NEXT_BEEFY_AUTHORITY_SET)
			.ok_or(Error::MissingNextAuthoritySet)?;

		Ok(BeefyStakesInfo {
			current_stakes,
			current_authority_set,
			next_stakes,
			next_authority_set,
		})
	}
}
