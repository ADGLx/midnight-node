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

//! [`AuxStoreDb`] — an implementation of the midnight-storage [`DB`] trait backed
//! by the node's auxiliary storage ([`LedgerBackendDb`]).
//!
//! Note: `midnight-storage-core` is always compiled with the `layout-v2` feature
//! in this workspace, so `OnDiskObject` has no `ref_count` field and the `DB`
//! trait has no `get_unreachable_keys` method.

use midnight_primitives_ledger::LedgerBackendDb;
use midnight_serialize::{Deserializable, Serializable};
use midnight_storage_core::{
	WellBehavedHasher,
	arena::ArenaHash,
	backend::OnDiskObject,
	db::{DB, DummyArbitrary, Update},
};
use sha2::digest::OutputSizeUser;
#[allow(deprecated)]
use sha2::digest::generic_array::GenericArray;
use std::{
	collections::HashMap,
	fmt::Debug,
	marker::PhantomData,
	sync::{
		Arc, RwLock,
		atomic::{AtomicUsize, Ordering},
	},
};

// Key prefixes for namespace separation in AuxStore.
const NODE_PREFIX: &[u8] = b"ldgr:n:";
const GC_ROOT_PREFIX: &[u8] = b"ldgr:r:";

// Meta keys for index recovery.
const META_ROOTS: &[u8] = b"ldgr:meta:roots";
const META_COUNT: &[u8] = b"ldgr:meta:count";

/// A [`DB`] implementation backed by the node's auxiliary storage.
pub struct AuxStoreDb<H: WellBehavedHasher = sha2::Sha256> {
	backend: Arc<dyn LedgerBackendDb>,
	roots: RwLock<HashMap<ArenaHash<H>, u32>>,
	node_count: AtomicUsize,
	_phantom: PhantomData<H>,
}

impl<H: WellBehavedHasher> Debug for AuxStoreDb<H> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("AuxStoreDb")
			.field("node_count", &self.node_count.load(Ordering::Relaxed))
			.finish()
	}
}

impl<H: WellBehavedHasher> DummyArbitrary for AuxStoreDb<H> {}

// --------------------------------------------------------------------------
// Helper: in-memory LedgerBackendDb for Default / tests
// --------------------------------------------------------------------------

struct InMemoryLedgerBackend {
	data: RwLock<HashMap<Vec<u8>, Vec<u8>>>,
}

impl Default for InMemoryLedgerBackend {
	fn default() -> Self {
		Self { data: RwLock::new(HashMap::new()) }
	}
}

impl LedgerBackendDb for InMemoryLedgerBackend {
	fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
		self.data.read().expect("lock poisoned").get(key).cloned()
	}

	fn write(&self, inserts: &[(&[u8], &[u8])], deletes: &[&[u8]]) -> Result<(), String> {
		let mut data = self.data.write().expect("lock poisoned");
		for (k, v) in inserts {
			data.insert(k.to_vec(), v.to_vec());
		}
		for k in deletes {
			data.remove(*k);
		}
		Ok(())
	}
}

impl<H: WellBehavedHasher> Default for AuxStoreDb<H> {
	fn default() -> Self {
		Self {
			backend: Arc::new(InMemoryLedgerBackend::default()),
			roots: RwLock::new(HashMap::new()),
			node_count: AtomicUsize::new(0),
			_phantom: PhantomData,
		}
	}
}

// --------------------------------------------------------------------------
// Key construction helpers
// --------------------------------------------------------------------------

fn make_key(prefix: &[u8], hash: &[u8]) -> Vec<u8> {
	let mut key = Vec::with_capacity(prefix.len() + hash.len());
	key.extend_from_slice(prefix);
	key.extend_from_slice(hash);
	key
}

fn node_key<H: WellBehavedHasher>(hash: &ArenaHash<H>) -> Vec<u8> {
	make_key(NODE_PREFIX, &hash.0)
}

fn gc_root_key<H: WellBehavedHasher>(hash: &ArenaHash<H>) -> Vec<u8> {
	make_key(GC_ROOT_PREFIX, &hash.0)
}

fn serialize_node<H: WellBehavedHasher>(node: &OnDiskObject<H>) -> Vec<u8> {
	let mut bytes = Vec::with_capacity(<OnDiskObject<H> as Serializable>::serialized_size(node));
	<OnDiskObject<H> as Serializable>::serialize(node, &mut bytes)
		.expect("Failed to serialize OnDiskObject");
	bytes
}

fn deserialize_node<H: WellBehavedHasher>(bytes: &[u8]) -> OnDiskObject<H> {
	OnDiskObject::<H>::deserialize(&mut &bytes[..], 0).expect("Failed to deserialize OnDiskObject")
}

fn bytes_to_arena_hash<H: WellBehavedHasher>(bytes: &[u8]) -> ArenaHash<H> {
	assert_eq!(
		bytes.len(),
		<H as OutputSizeUser>::output_size(),
		"incorrect length for arena hash"
	);
	#[allow(deprecated)]
	ArenaHash(GenericArray::from_iter(bytes.iter().copied()))
}

// --------------------------------------------------------------------------
// Index serialization (for meta keys)
// --------------------------------------------------------------------------

fn serialize_roots<H: WellBehavedHasher>(roots: &HashMap<ArenaHash<H>, u32>) -> Vec<u8> {
	let mut buf = Vec::new();
	let count = roots.len() as u64;
	buf.extend_from_slice(&count.to_le_bytes());
	for (hash, count) in roots {
		buf.extend_from_slice(&hash.0);
		buf.extend_from_slice(&count.to_le_bytes());
	}
	buf
}

fn deserialize_roots<H: WellBehavedHasher>(bytes: &[u8]) -> HashMap<ArenaHash<H>, u32> {
	let hash_size = <H as OutputSizeUser>::output_size();
	if bytes.len() < 8 {
		return HashMap::new();
	}
	let count = u64::from_le_bytes(bytes[..8].try_into().unwrap()) as usize;
	let entry_size = hash_size + 4;
	let mut map = HashMap::with_capacity(count);
	let data = &bytes[8..];
	for i in 0..count {
		let offset = i * entry_size;
		if offset + entry_size > data.len() {
			break;
		}
		let hash = bytes_to_arena_hash::<H>(&data[offset..offset + hash_size]);
		let root_count =
			u32::from_le_bytes(data[offset + hash_size..offset + entry_size].try_into().unwrap());
		map.insert(hash, root_count);
	}
	map
}

// --------------------------------------------------------------------------
// Construction
// --------------------------------------------------------------------------

impl<H: WellBehavedHasher> AuxStoreDb<H> {
	/// Create a new `AuxStoreDb` backed by the given [`LedgerBackendDb`].
	///
	/// Loads secondary indexes from meta keys to restore in-memory state.
	pub fn new(backend: Arc<dyn LedgerBackendDb>) -> Self {
		let roots = backend
			.get(META_ROOTS)
			.map(|bytes| deserialize_roots::<H>(&bytes))
			.unwrap_or_default();

		let node_count = backend
			.get(META_COUNT)
			.map(|bytes| u64::from_le_bytes(bytes.try_into().unwrap_or([0u8; 8])) as usize)
			.unwrap_or(0);

		Self {
			backend,
			roots: RwLock::new(roots),
			node_count: AtomicUsize::new(node_count),
			_phantom: PhantomData,
		}
	}

	/// Persist the in-memory indexes to the backend.
	pub fn persist_indexes(&self) {
		let roots = self.roots.read().expect("lock poisoned");
		let roots_bytes = serialize_roots::<H>(&roots);
		let count_bytes = (self.node_count.load(Ordering::Relaxed) as u64).to_le_bytes();

		let inserts: Vec<(&[u8], &[u8])> =
			vec![(META_ROOTS, &roots_bytes), (META_COUNT, &count_bytes)];

		if let Err(e) = self.backend.write(&inserts, &[]) {
			log::error!("Failed to persist ledger indexes: {e}");
		}
	}
}

// --------------------------------------------------------------------------
// DB trait implementation
// --------------------------------------------------------------------------

impl<H: WellBehavedHasher> DB for AuxStoreDb<H> {
	type Hasher = H;

	fn get_node(&self, key: &ArenaHash<Self::Hasher>) -> Option<OnDiskObject<Self::Hasher>> {
		let db_key = node_key(key);
		self.backend.get(&db_key).map(|bytes| deserialize_node(&bytes))
	}

	fn insert_node(&mut self, key: ArenaHash<Self::Hasher>, object: OnDiskObject<Self::Hasher>) {
		let db_key = node_key(&key);
		let value = serialize_node(&object);

		let inserts: Vec<(&[u8], &[u8])> = vec![(&db_key, &value)];

		// Check if this is a new node or update
		let existing = self.backend.get(&db_key);
		if existing.is_none() {
			self.node_count.fetch_add(1, Ordering::Relaxed);
		}

		self.backend.write(&inserts, &[]).expect("Failed to write node to backend");
	}

	fn delete_node(&mut self, key: &ArenaHash<Self::Hasher>) {
		let db_key = node_key(key);

		// Only decrement if the node actually existed
		if self.backend.get(&db_key).is_some() {
			self.node_count.fetch_sub(1, Ordering::Relaxed);
		}

		self.backend
			.write(&[], &[db_key.as_slice()])
			.expect("Failed to delete node from backend");
	}

	fn batch_update<I>(&mut self, iter: I)
	where
		I: Iterator<Item = (ArenaHash<Self::Hasher>, Update<Self::Hasher>)>,
	{
		let mut insert_bufs: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();
		let mut delete_bufs: Vec<Vec<u8>> = Vec::new();

		let mut roots = self.roots.write().expect("lock poisoned");

		for (key, update) in iter {
			match update {
				Update::InsertNode(object) => {
					let db_key = node_key(&key);
					// Check if new node
					if self.backend.get(&db_key).is_none() {
						self.node_count.fetch_add(1, Ordering::Relaxed);
					}
					let value = serialize_node(&object);
					insert_bufs.push((db_key, value));
				},
				Update::DeleteNode => {
					let db_key = node_key(&key);
					if self.backend.get(&db_key).is_some() {
						self.node_count.fetch_sub(1, Ordering::Relaxed);
					}
					delete_bufs.push(db_key);
				},
				Update::SetRootCount(count) => {
					let rk = gc_root_key(&key);
					if count == 0 {
						delete_bufs.push(rk);
						roots.remove(&key);
					} else {
						insert_bufs.push((rk, count.to_le_bytes().to_vec()));
						roots.insert(key, count);
					}
				},
			}
		}

		// Also persist the updated indexes
		let roots_bytes = serialize_roots::<H>(&roots);
		let count_bytes = (self.node_count.load(Ordering::Relaxed) as u64).to_le_bytes();
		insert_bufs.push((META_ROOTS.to_vec(), roots_bytes));
		insert_bufs.push((META_COUNT.to_vec(), count_bytes.to_vec()));

		let inserts: Vec<(&[u8], &[u8])> =
			insert_bufs.iter().map(|(k, v)| (k.as_slice(), v.as_slice())).collect();
		let deletes: Vec<&[u8]> = delete_bufs.iter().map(|k| k.as_slice()).collect();

		self.backend.write(&inserts, &deletes).expect("Failed to batch_update backend");
	}

	fn batch_get_nodes<I>(
		&self,
		keys: I,
	) -> Vec<(ArenaHash<Self::Hasher>, Option<OnDiskObject<Self::Hasher>>)>
	where
		I: Iterator<Item = ArenaHash<Self::Hasher>>,
	{
		keys.map(|k| {
			let node = self.get_node(&k);
			(k, node)
		})
		.collect()
	}

	fn get_root_count(&self, key: &ArenaHash<Self::Hasher>) -> u32 {
		// Check in-memory cache first
		let roots = self.roots.read().expect("lock poisoned");
		if let Some(&count) = roots.get(key) {
			return count;
		}
		drop(roots);

		// Fall back to backend
		let rk = gc_root_key(key);
		self.backend
			.get(&rk)
			.map(|bytes| {
				u32::from_le_bytes(bytes.try_into().expect("gc root count should be 4 bytes"))
			})
			.unwrap_or(0)
	}

	fn set_root_count(&mut self, key: ArenaHash<Self::Hasher>, count: u32) {
		let rk = gc_root_key(&key);

		let mut roots = self.roots.write().expect("lock poisoned");

		if count == 0 {
			roots.remove(&key);
			self.backend.write(&[], &[&rk]).expect("Failed to delete root count");
		} else {
			roots.insert(key, count);
			let value = count.to_le_bytes();
			self.backend.write(&[(&rk, &value)], &[]).expect("Failed to set root count");
		}
	}

	fn get_roots(&self) -> HashMap<ArenaHash<Self::Hasher>, u32> {
		self.roots.read().expect("lock poisoned").clone()
	}

	fn size(&self) -> usize {
		self.node_count.load(Ordering::Relaxed)
	}
}

/// Create a new in-memory [`LedgerBackendDb`] for testing purposes.
pub fn new_in_memory_backend() -> Arc<dyn LedgerBackendDb> {
	Arc::new(InMemoryLedgerBackend::default())
}

#[cfg(test)]
mod tests {
	use super::*;
	use sha2::Digest;

	type TestDb = AuxStoreDb<sha2::Sha256>;

	/// Create a test ArenaHash by hashing the given data.
	fn test_hash(data: &[u8]) -> ArenaHash<sha2::Sha256> {
		let hash = sha2::Sha256::digest(data);
		#[allow(deprecated)]
		ArenaHash(GenericArray::clone_from_slice(&hash))
	}

	#[test]
	fn root_count_operations() {
		let mut db = TestDb::default();

		let key = test_hash(b"root_node");
		assert_eq!(db.get_root_count(&key), 0);

		db.set_root_count(key.clone(), 5);
		assert_eq!(db.get_root_count(&key), 5);
		assert_eq!(db.get_roots().len(), 1);

		db.set_root_count(key.clone(), 0);
		assert_eq!(db.get_root_count(&key), 0);
		assert!(db.get_roots().is_empty());
	}

	#[test]
	fn persist_and_restore_indexes() {
		let backend = Arc::new(InMemoryLedgerBackend::default());

		let key = test_hash(b"persist_test");

		// Create DB, insert data, persist indexes
		{
			let mut db = AuxStoreDb::<sha2::Sha256>::new(backend.clone());
			db.set_root_count(key.clone(), 3);
			db.persist_indexes();
		}

		// Create new DB from same backend - indexes should be restored
		{
			let db = AuxStoreDb::<sha2::Sha256>::new(backend.clone());
			assert_eq!(db.get_root_count(&key), 3);
		}
	}
}
