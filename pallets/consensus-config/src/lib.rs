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
//! These values are set at genesis and serve as the canonical on-chain reference
//! for mainchain epoch configuration, enabling startup consistency checks and
//! config hash computation.

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::pallet_prelude::*;

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
}

#[cfg(test)]
mod tests {
	use super::*;
	use frame_support::derive_impl;
	use frame_system::mocking::MockUncheckedExtrinsic;
	use sp_io::TestExternalities;
	use sp_runtime::{BuildStorage, generic};

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
}
