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

//! The Ledger crate provide host functions for the Node runtime
//!
//! We make use of module-parameterization here, an un-intentional feature of Rust
//! See this example code: https://www.reddit.com/r/rust/comments/yrihwb/comment/ivuzmgt
//!
//! This means we can use the same code for two different versions of the ledger crate
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

#[cfg(feature = "std")]
pub mod aux_store_db;

#[cfg(feature = "std")]
pub mod json;

#[cfg(feature = "std")]
mod utils;

pub mod host_api;

#[path = "versions"]
pub mod ledger_7 {
	#[cfg(feature = "std")]
	pub(crate) use {
		base_crypto as base_crypto_local, coin_structure as coin_structure_local,
		ledger_storage as ledger_storage_local,
		midnight_node_ledger_helpers::ledger_7 as helpers_local,
		midnight_serialize as midnight_serialize_local, mn_ledger as mn_ledger_local,
		onchain_runtime as onchain_runtime_local, transient_crypto as transient_crypto_local,
		zswap as zswap_local,
	};

	#[allow(clippy::duplicate_mod)]
	#[path = "block_context/pre_ledger_8.rs"]
	mod block_context;
	pub use block_context::*;

	pub const CRATE_NAME: &str = "mn-ledger";
	#[allow(clippy::duplicate_mod)]
	mod common;
	pub use common::*;
}

#[path = "versions"]
pub mod ledger_8 {
	#[cfg(feature = "std")]
	pub(crate) use {
		base_crypto as base_crypto_local, coin_structure as coin_structure_local,
		ledger_storage_ledger_8 as ledger_storage_local,
		midnight_node_ledger_helpers::ledger_8 as helpers_local,
		midnight_serialize as midnight_serialize_local, mn_ledger_8 as mn_ledger_local,
		onchain_runtime_ledger_8 as onchain_runtime_local,
		transient_crypto as transient_crypto_local, zswap_ledger_8 as zswap_local,
	};

	#[path = "block_context/post_ledger_8.rs"]
	mod block_context;
	pub use block_context::*;

	pub const CRATE_NAME: &str = "mn-ledger-8";
	#[allow(clippy::duplicate_mod)]
	mod common;
	pub use common::*;
}

pub use ledger_8 as latest;

#[cfg(feature = "std")]
/// Drops all versioned default ledger storages.
///
/// Intended to be called from the embedding application shutdown path (for
/// example after Tokio/node shutdown completes) to ensure DB-backed storage is
/// released deterministically.
pub fn drop_all_default_storage() {
	// Drop AuxStoreDb-backed storage
	type LedgerDb = aux_store_db::AuxStoreDb<sha2::Sha256>;
	use ledger_storage_ledger_8::storage::{try_get_default_storage, unsafe_drop_default_storage};
	if try_get_default_storage::<LedgerDb>().is_some() {
		unsafe_drop_default_storage::<LedgerDb>();
	}

	// Also try dropping legacy ParityDb storage (for backward compatibility)
	ledger_7::storage::drop_default_storage_if_exists();
	ledger_8::storage::drop_default_storage_if_exists();
}

mod common;

pub mod types {
	pub use super::common::types::*;

	pub use super::host_api::ledger_8::ledger_8_bridge as active_ledger_bridge;
	pub use super::latest::types as active_version;
}

#[cfg(test)]
mod tests {
	use crate::aux_store_db::{AuxStoreDb, new_in_memory_backend};
	use frame_support::assert_ok;
	use ledger_storage_ledger_8::{
		Storage,
		storage::{set_default_storage, try_get_default_storage, unsafe_drop_default_storage},
	};

	type LedgerDb = AuxStoreDb<sha2::Sha256>;

	#[test]
	fn set_and_drop_default_storage() {
		{
			// Set default storage
			let res = set_default_storage(|| {
				let db: LedgerDb = AuxStoreDb::new(new_in_memory_backend());
				Storage::new(0, db)
			});

			assert_ok!(res);
		}

		// Drop default storage
		unsafe_drop_default_storage::<LedgerDb>();
		assert!(try_get_default_storage::<LedgerDb>().is_none());
	}
}
