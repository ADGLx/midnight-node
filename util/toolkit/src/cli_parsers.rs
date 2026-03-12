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

use std::str::FromStr;

use midnight_node_ledger_helpers::*;

use crate::tx_generator::source::FetchCacheConfig;

pub trait TokenDecode: Sized + Send + Sync + Clone {
	fn decode(token_id: [u8; 32]) -> Self;
}

impl TokenDecode for UnshieldedTokenType {
	fn decode(token_id: [u8; 32]) -> Self {
		UnshieldedTokenType(HashOutput(token_id))
	}
}

impl TokenDecode for ShieldedTokenType {
	fn decode(token_id: [u8; 32]) -> Self {
		ShieldedTokenType(HashOutput(token_id))
	}
}

pub fn token_decode<T: TokenDecode>(input: &str) -> Result<T, clap::error::Error> {
	let token_id: [u8; 32] = hex_str_decode(input)?;
	let token = T::decode(token_id);

	Ok(token)
}

pub fn wallet_seed_decode(input: &str) -> Result<WalletSeed, clap::error::Error> {
	input.parse().map_err(|e| {
		let mut err = clap::Error::new(clap::error::ErrorKind::ValueValidation);
		err.insert(
			clap::error::ContextKind::Custom,
			clap::error::ContextValue::String(format!("failed to parse seed: {}", e)),
		);
		err
	})
}

pub fn keypair_from_str(input: &str) -> Result<Keypair, clap::error::Error> {
	input.parse().map_err(|e| {
		let mut err = clap::Error::new(clap::error::ErrorKind::ValueValidation);
		err.insert(
			clap::error::ContextKind::Custom,
			clap::error::ContextValue::String(format!("failed to parse keypair: {}", e)),
		);
		err
	})
}

pub fn hex_ledger_decode<T: Deserializable + Tagged>(input: &str) -> Result<T, clap::error::Error> {
	hex_ledger_tagged_decode::<T>(input)
}

pub fn coin_public_decode(input: &str) -> Result<CoinPublicKey, clap::error::Error> {
	hex_ledger_prefer_tagged_decode(input)
}

pub fn contract_address_decode(input: &str) -> Result<ContractAddress, clap::error::Error> {
	hex_ledger_prefer_tagged_decode(input)
}

pub fn hex_ledger_prefer_tagged_decode<T>(input: &str) -> Result<T, clap::error::Error>
where
	T: Deserializable + Tagged,
{
	hex_ledger_tagged_decode::<T>(input).or_else(|_| hex_ledger_untagged_decode(input))
}

pub fn hex_ledger_untagged_decode<T>(input: &str) -> Result<T, clap::error::Error>
where
	T: Deserializable,
{
	let bytes = hex::decode(input).map_err(|e| {
		let mut err = clap::Error::new(clap::error::ErrorKind::ValueValidation);
		err.insert(
			clap::error::ContextKind::Custom,
			clap::error::ContextValue::String(format!("failed to parse hex input: {}", e)),
		);
		err
	})?;

	let mut cursor = &bytes[..];
	let res = <T as Deserializable>::deserialize(&mut cursor, 0).map_err(|e| {
		let mut err = clap::Error::new(clap::error::ErrorKind::ValueValidation);
		err.insert(
			clap::error::ContextKind::Custom,
			clap::error::ContextValue::String(format!("failed to deserialize arg: {e}")),
		);
		err
	})?;

	if !cursor.is_empty() {
		let mut err = clap::Error::new(clap::error::ErrorKind::ValueValidation);
		err.insert(
			clap::error::ContextKind::Custom,
			clap::error::ContextValue::String(format!(
				"unexpected trailing data: {} extra byte(s) after deserialization",
				cursor.len()
			)),
		);
		return Err(err);
	}

	Ok(res)
}

pub fn hex_ledger_tagged_decode<T>(input: &str) -> Result<T, clap::error::Error>
where
	T: Deserializable + Tagged,
{
	let bytes = hex::decode(input).map_err(|e| {
		let mut err = clap::Error::new(clap::error::ErrorKind::ValueValidation);
		err.insert(
			clap::error::ContextKind::Custom,
			clap::error::ContextValue::String(format!("failed to parse: {}", e)),
		);
		err
	})?;

	let res: T = deserialize(&mut &bytes[..]).map_err(|e| {
		let mut err = clap::Error::new(clap::error::ErrorKind::ValueValidation);
		err.insert(
			clap::error::ContextKind::Custom,
			clap::error::ContextValue::String(format!("failed to deserialize arg: {e}")),
		);
		err
	})?;

	Ok(res)
}

pub fn hex_bytes(input: &str) -> Result<Vec<u8>, clap::error::Error> {
	// Remove 0x prefix if present
	let hex_str = input.strip_prefix("0x").unwrap_or(input);
	hex::decode(hex_str).map_err(|e| {
		let mut err = clap::Error::new(clap::error::ErrorKind::ValueValidation);
		err.insert(
			clap::error::ContextKind::Custom,
			clap::error::ContextValue::String(format!("failed to parse seed: {}", e)),
		);
		err
	})
}

pub fn hex_str_decode<T>(input: &str) -> Result<T, clap::error::Error>
where
	T: TryFrom<Vec<u8>, Error = Vec<u8>>,
{
	let bytes = hex_bytes(input)?;

	let res: T = bytes.try_into().map_err(|e: Vec<u8>| {
		let mut err = clap::Error::new(clap::error::ErrorKind::ValueValidation);
		err.insert(
			clap::error::ContextKind::Custom,
			clap::error::ContextValue::String(format!(
				"incorrect length for token type string. Expected 32, got {}",
				e.len()
			)),
		);
		err
	})?;

	Ok(res)
}

pub fn fetch_cache_config(input: &str) -> Result<FetchCacheConfig, clap::Error> {
	FetchCacheConfig::from_str(input).map_err(|error| {
		let mut err = clap::Error::new(clap::error::ErrorKind::ValueValidation);

		err.insert(
			clap::error::ContextKind::Custom,
			clap::error::ContextValue::String(format!("invalid fetch cache config: {}", error)),
		);

		err
	})
}

pub fn wallet_address(input: &str) -> Result<WalletAddress, clap::Error> {
	WalletAddress::from_str(input).map_err(|error| {
		let mut err = clap::Error::new(clap::error::ErrorKind::ValueValidation);
		err.insert(
			clap::error::ContextKind::Custom,
			clap::error::ContextValue::String(format!("invalid wallet address: {}", error)),
		);

		err
	})
}

pub fn utxo_id_decode(input: &str) -> Result<UtxoId, clap::Error> {
	UtxoId::from_str(input).map_err(|error| {
		let mut err = clap::Error::new(clap::error::ErrorKind::ValueValidation);
		err.insert(
			clap::error::ContextKind::Custom,
			clap::error::ContextValue::String(format!("invalid utxo id: {}", error)),
		);

		err
	})
}

#[cfg(test)]
mod tests {
	use super::*;
	use hex::ToHex;

	const VALID_32_BYTES_HEX: &str =
		"dd76bcd08403b3a9af64485b1f3960437e7c5872c269d6d06d32b2f298d71577";

	fn make_contract_address() -> ContractAddress {
		let bytes = hex::decode(VALID_32_BYTES_HEX).unwrap();
		<ContractAddress as Deserializable>::deserialize(&mut &bytes[..], 0).unwrap()
	}

	fn make_coin_public_key() -> CoinPublicKey {
		let bytes = hex::decode(VALID_32_BYTES_HEX).unwrap();
		<CoinPublicKey as Deserializable>::deserialize(&mut &bytes[..], 0).unwrap()
	}

	#[test]
	fn coin_public_decode_accepts_untagged() {
		let result = coin_public_decode(VALID_32_BYTES_HEX);
		assert!(result.is_ok(), "expected untagged CoinPublicKey to be accepted");
	}

	#[test]
	fn coin_public_decode_accepts_tagged() {
		let key = make_coin_public_key();
		let tagged_hex: String = serialize(&key).unwrap().encode_hex();
		let result = coin_public_decode(&tagged_hex);
		assert!(result.is_ok(), "expected tagged CoinPublicKey to be accepted");
	}

	#[test]
	fn coin_public_decode_rejects_wrong_type_tag() {
		let addr = make_contract_address();
		let tagged_hex: String = serialize(&addr).unwrap().encode_hex();
		let result = coin_public_decode(&tagged_hex);
		assert!(result.is_err(), "expected wrong type tag to be rejected");
	}

	#[test]
	fn coin_public_decode_rejects_trailing_bytes() {
		let with_extra = format!("{}aabb", VALID_32_BYTES_HEX);
		let result = coin_public_decode(&with_extra);
		assert!(result.is_err(), "expected trailing bytes to be rejected");
	}

	#[test]
	fn contract_address_decode_accepts_untagged() {
		let result = contract_address_decode(VALID_32_BYTES_HEX);
		assert!(result.is_ok(), "expected untagged ContractAddress to be accepted");
	}

	#[test]
	fn contract_address_decode_accepts_tagged() {
		let addr = make_contract_address();
		let tagged_hex: String = serialize(&addr).unwrap().encode_hex();
		let result = contract_address_decode(&tagged_hex);
		assert!(result.is_ok(), "expected tagged ContractAddress to be accepted");
	}

	#[test]
	fn contract_address_decode_rejects_trailing_bytes() {
		let with_extra = format!("{}aabb", VALID_32_BYTES_HEX);
		let result = contract_address_decode(&with_extra);
		assert!(result.is_err(), "expected trailing bytes to be rejected");
	}

	#[test]
	fn hash_output_decode_accepts_valid() {
		let result = hex_ledger_untagged_decode::<HashOutput>(VALID_32_BYTES_HEX);
		assert!(result.is_ok(), "expected valid HashOutput to be accepted");
	}

	#[test]
	fn hash_output_decode_rejects_trailing_bytes() {
		let with_extra = format!("{}ff", VALID_32_BYTES_HEX);
		let result = hex_ledger_untagged_decode::<HashOutput>(&with_extra);
		assert!(result.is_err(), "expected trailing bytes to be rejected for HashOutput");
	}

	#[test]
	fn invalid_hex_rejected() {
		assert!(coin_public_decode("zzzz").is_err());
		assert!(contract_address_decode("zzzz").is_err());
		assert!(hex_ledger_untagged_decode::<HashOutput>("zzzz").is_err());
	}

	#[test]
	fn empty_input_rejected() {
		assert!(coin_public_decode("").is_err());
		assert!(contract_address_decode("").is_err());
		assert!(hex_ledger_untagged_decode::<HashOutput>("").is_err());
	}
}
