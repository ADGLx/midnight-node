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

pub mod types;

#[cfg(feature = "std")]
use std::sync::atomic::{AtomicU64, Ordering};

/// Tracks the highest block number we've processed.
/// When replaying old blocks (block_number <= highest_seen), we shuffle UTXO
/// segment order to probabilistically match the old HashMap iteration order.
/// For new blocks (block_number > highest_seen), we use deterministic BTreeMap order.
#[cfg(feature = "std")]
static HIGHEST_BLOCK_SEEN: AtomicU64 = AtomicU64::new(0);

#[cfg(feature = "std")]
pub fn set_highest_block_seen(n: u64) {
	HIGHEST_BLOCK_SEEN.store(n, Ordering::Relaxed);
}

#[cfg(feature = "std")]
pub fn highest_block_seen() -> u64 {
	HIGHEST_BLOCK_SEEN.load(Ordering::Relaxed)
}
