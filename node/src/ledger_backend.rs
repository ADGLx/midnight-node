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

//! Newtype wrapper around the node's Substrate backend that implements
//! [`LedgerBackendDb`].

use midnight_primitives_ledger::LedgerBackendDb;
use sc_client_api::backend::AuxStore;
use std::sync::Arc;

use crate::service::FullBackend;

/// Thin wrapper that satisfies the orphan rule so we can implement
/// [`LedgerBackendDb`] (defined in `midnight-primitives-ledger`) for the
/// Substrate backend (defined in `sc-client-db`).
pub struct BackendLedgerDb(pub Arc<FullBackend>);

impl LedgerBackendDb for BackendLedgerDb {
	fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
		AuxStore::get_aux(self.0.as_ref(), key).ok().flatten()
	}

	fn write(&self, inserts: &[(&[u8], &[u8])], deletes: &[&[u8]]) -> Result<(), String> {
		AuxStore::insert_aux(self.0.as_ref(), inserts, deletes).map_err(|e| e.to_string())
	}
}
