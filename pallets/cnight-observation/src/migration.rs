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

pub mod v1 {
	use crate::pallet::{Config, NextCardanoPosition, Pallet, UtxoOwners};
	use frame_support::{
		pallet_prelude::*,
		storage::{storage_prefix, unhashed},
		traits::{GetStorageVersion, OnRuntimeUpgrade},
	};

	/// Migration from v0 to v1: removes redemption-related storage.
	///
	/// Clears:
	/// - `MainChainRedemptionValidatorAddress` (removed storage item)
	/// - `UtxoOwners` (contains nonces from old redemption_create prefix)
	/// - `NextCardanoPosition` (reset to trigger full replay from genesis)
	pub struct RemoveRedemptionStorage<T: Config>(core::marker::PhantomData<T>);

	impl<T: Config> OnRuntimeUpgrade for RemoveRedemptionStorage<T> {
		fn on_runtime_upgrade() -> Weight {
			let on_chain = Pallet::<T>::on_chain_storage_version();
			if on_chain >= 1 {
				log::info!("CNight observation migration v1 already applied, skipping");
				return Weight::zero();
			}

			log::info!("Running CNight observation migration v0 -> v1");

			// Clear the removed MainChainRedemptionValidatorAddress storage
			let key = storage_prefix(b"CNightObservation", b"MainChainRedemptionValidatorAddress");
			unhashed::kill(&key);

			// Clear UtxoOwners completely (contains nonces derived from old redemption_create prefix)
			let removed = UtxoOwners::<T>::clear(u32::MAX, None).unique;

			// Reset NextCardanoPosition to trigger full replay
			NextCardanoPosition::<T>::kill();

			log::info!(
				"CNight observation migration v1 complete: removed {removed} utxo owner entries"
			);

			StorageVersion::new(1).put::<Pallet<T>>();

			T::DbWeight::get().reads_writes(1, removed as u64 + 3)
		}
	}
}
