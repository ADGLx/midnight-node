#![allow(dead_code)]

use midnight_primitives_beefy::{
	BeefyStakes,
	known_payloads::{
		CURRENT_BEEFY_AUTHORITY_SET, CURRENT_BEEFY_STAKES_ID, NEXT_BEEFY_AUTHORITY_SET,
		NEXT_BEEFY_STAKES_ID,
	},
};
use sp_consensus_beefy::{
	BeefyPayloadId, Payload as BeefyPayload,
	ecdsa_crypto::AuthorityId as BeefyId,
	mmr::{BeefyAuthoritySet, BeefyNextAuthoritySet},
};
use sp_core::H256;

use crate::{Error, cardano_encoding::Payload};
#[derive(Debug)]
pub struct BeefyStakesInfo {
	pub current_stakes: BeefyStakes<BeefyId>,
	pub current_authority_set: BeefyAuthoritySet<H256>,
	pub next_stakes: BeefyStakes<BeefyId>,
	pub next_authority_set: BeefyNextAuthoritySet<H256>,
	pub current_stakes: BeefyStakes<BeefyId>,
	pub current_authority_set: BeefyAuthoritySet<H256>,
	pub next_stakes: BeefyStakes<BeefyId>,
	pub next_authority_set: BeefyNextAuthoritySet<H256>,
}

impl TryFrom<BeefyPayload> for BeefyStakesInfo {
	type Error = Error;

	fn try_from(value: BeefyPayload) -> Result<Self, Self::Error> {
		BeefyStakesInfo::try_from(&value)
	}
}

impl TryFrom<&BeefyPayload> for BeefyStakesInfo {
	type Error = Error;

	fn try_from(value: &BeefyPayload) -> Result<Self, Self::Error> {
		let current_stakes: BeefyStakes<BeefyId> = value
			.get_decoded(&CURRENT_BEEFY_STAKES_ID)
			.ok_or(Error::MissingCurrentBeefyStakes)?;
		let current_authority_set: BeefyAuthoritySet<H256> = value
			.get_decoded(&CURRENT_BEEFY_AUTHORITY_SET)
			.ok_or(Error::MissingCurrentAuthoritySet)?;

		let next_stakes: BeefyStakes<BeefyId> =
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

/// Returns all the payloads from a list of ids
pub fn extract_all_payloads(payload: &BeefyPayload, ids: Vec<BeefyPayloadId>) -> Vec<Payload> {
	ids.into_iter().fold(vec![], |accumulated_payloads, id| {
		let mut new_payloads = extract_specific_payloads(payload, id);
		// append the accumulated payloads to this new list
		new_payloads.extend_from_slice(accumulated_payloads.as_slice());

		// return the newly accumulated payloads
		new_payloads
	})
}

/// Get all payloads from the given id
fn extract_specific_payloads(payload: &BeefyPayload, id: BeefyPayloadId) -> Vec<Payload> {
	payload
		.get_all_raw(&id)
		.map(|raw_vec| Payload { id: id.to_vec(), data: raw_vec.clone() })
		.collect()
}

#[cfg(test)]
mod test {
	use midnight_primitives_beefy::known_payloads::{
		CURRENT_BEEFY_AUTHORITY_SET, CURRENT_BEEFY_STAKES_ID, NEXT_BEEFY_AUTHORITY_SET,
		NEXT_BEEFY_STAKES_ID,
	};
	use sp_consensus_beefy::Payload as BeefyPayload;
	use sp_core::{H256, bytes::from_hex};

	use crate::{
		cardano_encoding::Payload,
		helper::{
			HexExt,
			test::{ECDSA_ALICE, ECDSA_BOB, ECDSA_CHARLIE, ECDSA_DAVE, decode, get_ecdsa},
		},
		justification::{BeefyStakesInfo, extract_all_payloads, extract_specific_payloads},
	};

	#[test]
	fn test_extract_beefy_stakes() {
		let encoded_payload = "0x146362b000000000000000000400000086fd5cd50b8bb99aa5c8befc197dd8273d17a4530b44e7aca182a4af271bd6a86373950210020a1091341fe5664bfa1782d5e04779689068c916b04cb365ec3153755684d9a101000000000000000390084fdbf27d2b79d26a4f13f0ccd982cb755a661969143c37cbc49ef5b91f2701000000000000000389411795514af1627765eceffcbd002719f031604fadd7d188e2dc585b4e1afb000000000000000003bc9d0ca094bd5b8b3225d7651eac5d18c1c04bf8ae8f8b263eebca4e1410ed0c00000000000000006d6880850783e47991669df4fa44075cd0fa5d8532d2a99fce644fcc33c7395522c8526e62b001000000000000000400000086fd5cd50b8bb99aa5c8befc197dd8273d17a4530b44e7aca182a4af271bd6a86e73950210020a1091341fe5664bfa1782d5e04779689068c916b04cb365ec3153755684d9a101000000000000000390084fdbf27d2b79d26a4f13f0ccd982cb755a661969143c37cbc49ef5b91f2701000000000000000389411795514af1627765eceffcbd002719f031604fadd7d188e2dc585b4e1afb000000000000000003bc9d0ca094bd5b8b3225d7651eac5d18c1c04bf8ae8f8b263eebca4e1410ed0c0000000000000000";
		let encoded_root = "0x86fd5cd50b8bb99aa5c8befc197dd8273d17a4530b44e7aca182a4af271bd6a8";

		let payload = decode::<BeefyPayload>(encoded_payload);

		let stakes_info =
			BeefyStakesInfo::try_from(&payload).expect("should return BeefyStakesInfo");

		let expected_root = decode::<H256>(encoded_root);
		let current_authority_set = stakes_info.current_authority_set;
		assert_eq!(current_authority_set.keyset_commitment, expected_root.clone());
		assert_eq!(current_authority_set.len, 4);

		assert!(stakes_info.current_stakes.contains(&(get_ecdsa(ECDSA_ALICE), 1)));
		assert!(stakes_info.current_stakes.contains(&(get_ecdsa(ECDSA_BOB), 1)));
		assert!(stakes_info.current_stakes.contains(&(get_ecdsa(ECDSA_CHARLIE), 0)));
		assert!(stakes_info.current_stakes.contains(&(get_ecdsa(ECDSA_DAVE), 0)));

		let next_authority_set = stakes_info.next_authority_set;
		assert_eq!(next_authority_set.keyset_commitment, expected_root);
		assert_eq!(next_authority_set.id, 1);

		assert_eq!(stakes_info.next_stakes.len(), 4);
	}

	#[test]
	fn test_extract_payloads() {
		let encoded_payload = "0x146362b0000000000000000004000000147c6b950692c12a2fe19a006ec8ed9b37efea4e7f39d37195bdccb6f9b8ffbc6373950210020a1091341fe5664bfa1782d5e04779689068c916b04cb365ec3153755684d9a100000000000000000390084fdbf27d2b79d26a4f13f0ccd982cb755a661969143c37cbc49ef5b91f2701000000000000000389411795514af1627765eceffcbd002719f031604fadd7d188e2dc585b4e1afb000000000000000003bc9d0ca094bd5b8b3225d7651eac5d18c1c04bf8ae8f8b263eebca4e1410ed0c00000000000000006d6880f970ffcd6d99b8926bab7cb144f4ed77853086ed60ea370865be67fa0ebe28da6e62b0010000000000000004000000fd227ad55dad55395b2c5f5498c5fe83555119b8123432559bfa077f743c25bd6e73950210020a1091341fe5664bfa1782d5e04779689068c916b04cb365ec3153755684d9a101000000000000000390084fdbf27d2b79d26a4f13f0ccd982cb755a661969143c37cbc49ef5b91f2700000000000000000389411795514af1627765eceffcbd002719f031604fadd7d188e2dc585b4e1afb000000000000000003bc9d0ca094bd5b8b3225d7651eac5d18c1c04bf8ae8f8b263eebca4e1410ed0c0000000000000000";
		let payload = decode::<BeefyPayload>(encoded_payload);

		// --------- test current authority set
		let expected_curr_auth_set = "0x000000000000000004000000147c6b950692c12a2fe19a006ec8ed9b37efea4e7f39d37195bdccb6f9b8ffbc";
		let curr_auth_set = extract_specific_payloads(&payload, CURRENT_BEEFY_AUTHORITY_SET);
		assert_eq!(curr_auth_set.len(), 1);

		let curr_auth_set = curr_auth_set[0].data.as_hex();
		assert_eq!(&curr_auth_set, expected_curr_auth_set);

		// ------- test next stakes
		let expected_next_stakes = "0x10020a1091341fe5664bfa1782d5e04779689068c916b04cb365ec3153755684d9a101000000000000000390084fdbf27d2b79d26a4f13f0ccd982cb755a661969143c37cbc49ef5b91f2700000000000000000389411795514af1627765eceffcbd002719f031604fadd7d188e2dc585b4e1afb000000000000000003bc9d0ca094bd5b8b3225d7651eac5d18c1c04bf8ae8f8b263eebca4e1410ed0c0000000000000000";
		let next_stakes = extract_specific_payloads(&payload, NEXT_BEEFY_STAKES_ID);
		assert_eq!(next_stakes.len(), 1);

		let next_stakes = next_stakes[0].data.as_hex();
		assert_eq!(&next_stakes, expected_next_stakes);

		// ------- test remaining payload
		let other_payloads =
			extract_all_payloads(&payload, vec![CURRENT_BEEFY_STAKES_ID, NEXT_BEEFY_AUTHORITY_SET]);
		assert_eq!(other_payloads.len(), 2);

		// ------- test current stakes
		let expected_curr_stakes = from_hex("0x10020a1091341fe5664bfa1782d5e04779689068c916b04cb365ec3153755684d9a100000000000000000390084fdbf27d2b79d26a4f13f0ccd982cb755a661969143c37cbc49ef5b91f2701000000000000000389411795514af1627765eceffcbd002719f031604fadd7d188e2dc585b4e1afb000000000000000003bc9d0ca094bd5b8b3225d7651eac5d18c1c04bf8ae8f8b263eebca4e1410ed0c0000000000000000")
		.expect("failed to decode curr stakes hex");

		let expected_curr_stakes =
			Payload { id: CURRENT_BEEFY_STAKES_ID.to_vec(), data: expected_curr_stakes };
		assert!(other_payloads.contains(&expected_curr_stakes));

		// ------- test next authority set
		let expected_next_auth_set = from_hex(
			"0x010000000000000004000000fd227ad55dad55395b2c5f5498c5fe83555119b8123432559bfa077f743c25bd",
		)
		.expect("failed to decode next auth set hex");

		assert!(other_payloads.contains(&Payload {
			id: NEXT_BEEFY_AUTHORITY_SET.to_vec(),
			data: expected_next_auth_set,
		}));
	}
}
