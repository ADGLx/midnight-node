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

//! # Consensus Configuration Pallet
//!
//! Stores Cardano mainchain epoch parameters as decomposed primitive fields.
//! `MainchainEpochConfig` from partner-chains is NOT SCALE-encodable, so this
//! pallet stores each field individually as a `StorageValue`.
//!
//! These values are set at genesis and serve as the canonical on-chain
//! reference for mainchain epoch configuration. Existing chains that
//! receive the pallet via runtime upgrade will have uninitialized (zero)
//! storage; the `on_initialize` hook and startup consistency check handle
//! this gracefully.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::vec::Vec;
use parity_scale_codec::Encode;
use sp_runtime::{ConsensusEngineId, DigestItem, generic::OpaqueDigestItemId};

pub use pallet::*;

pub const CONSENSUS_CONFIG_ENGINE_ID: ConsensusEngineId = *b"MNCC";

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	#[pallet::getter(fn mc_epoch_duration_millis)]
	pub type McEpochDurationMillis<T: Config> = StorageValue<_, u64, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn mc_slot_duration_millis)]
	pub type McSlotDurationMillis<T: Config> = StorageValue<_, u64, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn mc_first_epoch_timestamp_millis)]
	pub type McFirstEpochTimestampMillis<T: Config> = StorageValue<_, u64, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn mc_first_epoch_number)]
	pub type McFirstEpochNumber<T: Config> = StorageValue<_, u32, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn mc_first_slot_number)]
	pub type McFirstSlotNumber<T: Config> = StorageValue<_, u64, ValueQuery>;

	#[pallet::genesis_config]
	#[derive(frame_support::DefaultNoBound)]
	pub struct GenesisConfig<T: Config> {
		pub mc_epoch_duration_millis: u64,
		pub mc_slot_duration_millis: u64,
		pub mc_first_epoch_timestamp_millis: u64,
		pub mc_first_epoch_number: u32,
		pub mc_first_slot_number: u64,
		#[serde(skip)]
		pub _config: core::marker::PhantomData<T>,
	}

	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			McEpochDurationMillis::<T>::put(self.mc_epoch_duration_millis);
			McSlotDurationMillis::<T>::put(self.mc_slot_duration_millis);
			McFirstEpochTimestampMillis::<T>::put(self.mc_first_epoch_timestamp_millis);
			McFirstEpochNumber::<T>::put(self.mc_first_epoch_number);
			McFirstSlotNumber::<T>::put(self.mc_first_slot_number);
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		/// Deposits a `DigestItem::Consensus(MNCC, config_hash)` each block.
		///
		/// Weight is estimated inline rather than via a `WeightInfo` benchmarking
		/// module because the hook has constant, deterministic cost: a fixed number
		/// of storage reads and one digest write with no input-dependent variation.
		fn on_initialize(_n: BlockNumberFor<T>) -> Weight {
			if !Self::is_initialized() {
				return T::DbWeight::get().reads(1);
			}

			let hash = Self::compute_config_hash();
			let log = DigestItem::Consensus(CONSENSUS_CONFIG_ENGINE_ID, hash.to_vec());
			<frame_system::Pallet<T>>::deposit_log(log);

			T::DbWeight::get().reads_writes(5, 1)
		}
	}

	impl<T: Config> Pallet<T> {
		/// Returns `true` when storage has been populated via genesis config.
		/// Uses `McEpochDurationMillis > 0` as sentinel — zero is never a valid
		/// epoch duration for any Cardano network.
		pub fn is_initialized() -> bool {
			McEpochDurationMillis::<T>::get() > 0
		}

		/// Computes the blake2-256 hash of the canonical SCALE-encoded config.
		/// Fields are encoded in a fixed order matching the storage declaration.
		pub fn compute_config_hash() -> [u8; 32] {
			let mut payload = Vec::new();
			McEpochDurationMillis::<T>::get().encode_to(&mut payload);
			McSlotDurationMillis::<T>::get().encode_to(&mut payload);
			McFirstEpochTimestampMillis::<T>::get().encode_to(&mut payload);
			McFirstEpochNumber::<T>::get().encode_to(&mut payload);
			McFirstSlotNumber::<T>::get().encode_to(&mut payload);
			sp_crypto_hashing::blake2_256(&payload)
		}

		/// Extracts the config hash from a `DigestItem::Consensus(MNCC, _)` entry.
		pub fn decode_config_hash(item: &DigestItem) -> Option<[u8; 32]> {
			item.try_to::<[u8; 32]>(OpaqueDigestItemId::Consensus(&CONSENSUS_CONFIG_ENGINE_ID))
		}
	}
}

sp_api::decl_runtime_apis! {
	/// Runtime API for reading consensus configuration from on-chain storage.
	pub trait ConsensusConfigApi {
		fn mc_epoch_duration_millis() -> u64;
		fn mc_slot_duration_millis() -> u64;
		fn mc_first_epoch_timestamp_millis() -> u64;
		fn mc_first_epoch_number() -> u32;
		fn mc_first_slot_number() -> u64;
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use frame_support::{derive_impl, traits::Hooks};
	use frame_system::mocking::MockUncheckedExtrinsic;
	use sp_io::TestExternalities;
	use sp_runtime::{BuildStorage, DigestItem, generic, traits::Header as HeaderT};

	type Header = generic::Header<u64, sp_runtime::traits::BlakeTwo256>;
	type Block = generic::Block<Header, MockUncheckedExtrinsic<Test>>;

	frame_support::construct_runtime!(
		pub enum Test {
			System: frame_system,
			ConsensusConfig: pallet,
		}
	);

	#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
	impl frame_system::Config for Test {
		type Block = Block;
	}

	impl pallet::Config for Test {}

	fn new_test_ext(genesis: pallet::GenesisConfig<Test>) -> TestExternalities {
		let mut t = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();
		genesis.assimilate_storage(&mut t).unwrap();
		TestExternalities::new(t)
	}

	fn test_genesis() -> pallet::GenesisConfig<Test> {
		pallet::GenesisConfig {
			mc_epoch_duration_millis: 432_000_000,
			mc_slot_duration_millis: 1_000,
			mc_first_epoch_timestamp_millis: 1_596_399_616_000,
			mc_first_epoch_number: 75,
			mc_first_slot_number: 86_400,
			_config: Default::default(),
		}
	}

	fn empty_test_ext() -> TestExternalities {
		let t = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();
		TestExternalities::new(t)
	}

	#[test]
	fn genesis_config_stores_all_fields() {
		let genesis = test_genesis();
		new_test_ext(genesis).execute_with(|| {
			assert_eq!(pallet::Pallet::<Test>::mc_epoch_duration_millis(), 432_000_000);
			assert_eq!(pallet::Pallet::<Test>::mc_slot_duration_millis(), 1_000);
			assert_eq!(
				pallet::Pallet::<Test>::mc_first_epoch_timestamp_millis(),
				1_596_399_616_000
			);
			assert_eq!(pallet::Pallet::<Test>::mc_first_epoch_number(), 75);
			assert_eq!(pallet::Pallet::<Test>::mc_first_slot_number(), 86_400);
		});
	}

	#[test]
	fn default_genesis_has_zero_values() {
		let genesis = pallet::GenesisConfig::<Test>::default();
		new_test_ext(genesis).execute_with(|| {
			assert_eq!(pallet::Pallet::<Test>::mc_epoch_duration_millis(), 0);
			assert_eq!(pallet::Pallet::<Test>::mc_slot_duration_millis(), 0);
			assert_eq!(pallet::Pallet::<Test>::mc_first_epoch_timestamp_millis(), 0);
			assert_eq!(pallet::Pallet::<Test>::mc_first_epoch_number(), 0);
			assert_eq!(pallet::Pallet::<Test>::mc_first_slot_number(), 0);
		});
	}

	#[test]
	fn storage_values_persist_independently() {
		let genesis = test_genesis();
		new_test_ext(genesis).execute_with(|| {
			assert_eq!(pallet::Pallet::<Test>::mc_epoch_duration_millis(), 432_000_000);
			assert_eq!(pallet::Pallet::<Test>::mc_first_epoch_number(), 75);
			assert_eq!(pallet::Pallet::<Test>::mc_first_slot_number(), 86_400);
		});
	}

	#[test]
	fn is_initialized_returns_false_when_empty() {
		empty_test_ext().execute_with(|| {
			assert!(!pallet::Pallet::<Test>::is_initialized());
		});
	}

	#[test]
	fn is_initialized_returns_true_after_genesis() {
		let genesis = test_genesis();
		new_test_ext(genesis).execute_with(|| {
			assert!(pallet::Pallet::<Test>::is_initialized());
		});
	}

	// TC-10: on_initialize deposits DigestItem::Consensus with config hash
	#[test]
	fn on_initialize_deposits_config_hash_digest() {
		let genesis = test_genesis();
		new_test_ext(genesis).execute_with(|| {
			<pallet::Pallet<Test> as Hooks<u64>>::on_initialize(1);
			let header: Header = System::finalize();

			let hash = header
				.digest()
				.convert_first(pallet::Pallet::<Test>::decode_config_hash)
				.expect("MNCC digest entry should be present");

			let expected_hash = pallet::Pallet::<Test>::compute_config_hash();
			assert_eq!(hash, expected_hash);
		});
	}

	// TC-16: on_initialize skips hash when storage is uninitialized
	#[test]
	fn on_initialize_skips_when_uninitialized() {
		empty_test_ext().execute_with(|| {
			<pallet::Pallet<Test> as Hooks<u64>>::on_initialize(1);
			let header: Header = System::finalize();

			let result = header.digest().convert_first(pallet::Pallet::<Test>::decode_config_hash);

			assert!(result.is_none(), "No MNCC digest when uninitialized");
		});
	}

	// TC-11: Config hash is deterministic
	#[test]
	fn config_hash_is_deterministic() {
		let genesis = test_genesis();
		new_test_ext(genesis).execute_with(|| {
			let hash1 = pallet::Pallet::<Test>::compute_config_hash();
			let hash2 = pallet::Pallet::<Test>::compute_config_hash();
			assert_eq!(hash1, hash2);
		});
	}

	#[test]
	fn config_hash_changes_with_different_values() {
		let genesis = test_genesis();
		new_test_ext(genesis).execute_with(|| {
			let hash_before = pallet::Pallet::<Test>::compute_config_hash();

			pallet::McSlotDurationMillis::<Test>::put(2_000u64);
			let hash_after = pallet::Pallet::<Test>::compute_config_hash();

			assert_ne!(hash_before, hash_after);
		});
	}

	#[test]
	fn decode_config_hash_roundtrips_known_hash() {
		let genesis = test_genesis();
		new_test_ext(genesis).execute_with(|| {
			let hash = pallet::Pallet::<Test>::compute_config_hash();
			let item = DigestItem::Consensus(super::CONSENSUS_CONFIG_ENGINE_ID, hash.to_vec());
			let decoded = pallet::Pallet::<Test>::decode_config_hash(&item);
			assert_eq!(decoded, Some(hash));
		});
	}

	#[test]
	fn decode_config_hash_returns_none_for_other_engine() {
		let item = DigestItem::Consensus(*b"ABCD", vec![1, 2, 3]);
		assert!(pallet::Pallet::<Test>::decode_config_hash(&item).is_none());
	}
}
