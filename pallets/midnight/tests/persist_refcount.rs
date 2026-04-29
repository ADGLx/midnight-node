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
//
// Integration test for the persist refcount accounting in apply_transaction
// and post_block_update. Lives in `tests/` rather than `src/tests.rs` so it
// runs in its own test binary, isolating the global ledger storage backend
// from other pallet tests.

use frame_support::{assert_ok, traits::OnFinalize};
use midnight_node_ledger::types::active_version::BlockContext;
use midnight_node_res::{
	networks::{MidnightNetwork, UndeployedNetwork},
	undeployed::transactions::{CHECK_TX, DEPLOY_TX, STORE_TX},
};
use pallet_midnight::{
	Call as MidnightCall,
	mock::{self, RuntimeOrigin, Test},
};
use sp_runtime::{traits::ValidateUnsigned, transaction_validity::TransactionSource};

fn init_ledger_state(block_context: BlockContext) {
	let path_buf = tempfile::tempdir().unwrap().keep();
	let state_key = midnight_node_ledger::latest::storage::init_storage_paritydb_separate(
		&path_buf,
		UndeployedNetwork.genesis_state(),
		1024 * 1024,
	);

	sp_tracing::try_init_simple();
	mock::Midnight::initialize_state(UndeployedNetwork.id(), &state_key);
	mock::System::set_block_number(1);
	mock::Timestamp::set_timestamp(block_context.tblock * 1000);
}

fn process_block(block_number: u64, block_context: BlockContext) {
	mock::Midnight::on_finalize(block_number);
	mock::System::set_block_number(block_number + 1);
	mock::Timestamp::set_timestamp(block_context.tblock * 1000);
}

fn root_count(state_key: &[u8]) -> Option<u32> {
	midnight_node_ledger::latest::storage::get_state_root_count(state_key)
}

fn current_state_key() -> Vec<u8> {
	pallet_midnight::StateKey::<Test>::get()
}

/// Walks an entire block lifecycle and asserts the persist/unpersist arithmetic
/// at every transition:
///   - within a block, intermediate apply_tx outputs are unrooted by their
///     successor (only the current tip is at refcount 1),
///   - post_block_update lands the post-block state at refcount 2 (regular +
///     extra persist) so it survives the next block's first apply unpersist,
///   - the next block's first apply drops the prior post-block state from
///     refcount 2 to 1, where it stays — alive for history/RPC.
///
/// Also guards that `validate_unsigned` and `pre_dispatch` are read-only — they
/// must not change the refcount of the current tip, since the next dispatch
/// needs to load that same state.
///
/// Single test fn: this integration test binary's tests share the global
/// default ledger storage, and absolute refcount assertions (e.g. "genesis is
/// rc=1") would be polluted by another test in the same binary alloc'ing the
/// same genesis. Sequencing every scenario inside one test keeps the assertions
/// precise without pulling in serial_test or a Mutex guard.
#[test]
fn persist_refcount_invariants() {
	let (deploy_tx, deploy_ctx) =
		midnight_node_ledger_helpers::ledger_8::extract_tx_with_context(DEPLOY_TX);
	let (store_tx, store_ctx) =
		midnight_node_ledger_helpers::ledger_8::extract_tx_with_context(STORE_TX);
	let (check_tx, check_ctx) =
		midnight_node_ledger_helpers::ledger_8::extract_tx_with_context(CHECK_TX);

	let deploy_call = MidnightCall::<Test>::send_mn_transaction { midnight_tx: deploy_tx.clone() };

	mock::new_test_ext().execute_with(|| {
		init_ledger_state(deploy_ctx.clone().into());

		let genesis_key = current_state_key();
		assert_eq!(
			root_count(&genesis_key),
			Some(2),
			"genesis is the block-0 post-block state, persisted at rc=2 by alloc_with_initial_state"
		);

		// Read-only guard 1: validate_unsigned + pre_dispatch on the genesis state
		// (rc=2). DEPLOY is valid here, so we expect Ok; the assertion that
		// matters is that rc is untouched.
		assert_ok!(<mock::Midnight as ValidateUnsigned>::validate_unsigned(
			TransactionSource::External,
			&deploy_call
		));
		assert_eq!(current_state_key(), genesis_key);
		assert_eq!(root_count(&genesis_key), Some(2), "validate_unsigned must not change tip rc");
		assert_ok!(<mock::Midnight as ValidateUnsigned>::pre_dispatch(&deploy_call));
		assert_eq!(current_state_key(), genesis_key);
		assert_eq!(root_count(&genesis_key), Some(2), "pre_dispatch must not change tip rc");

		// Block 1: apply DEPLOY.
		assert_ok!(mock::Midnight::send_mn_transaction(RuntimeOrigin::none(), deploy_tx));
		let post_deploy_key = current_state_key();
		assert_ne!(post_deploy_key, genesis_key);
		assert_eq!(
			root_count(&post_deploy_key),
			Some(1),
			"new state should be persisted at rc=1 after apply_transaction"
		);
		assert_eq!(
			root_count(&genesis_key),
			Some(1),
			"genesis drops from rc=2 to rc=1 after block 1's first apply, retained for history"
		);

		// Finalize block 1. post_block_update applies, persists once, unpersists
		// predecessor, then persists again — landing the post-block state at rc=2.
		process_block(2, store_ctx.clone().into());
		let post_block_1_key = current_state_key();
		assert_ne!(post_block_1_key, post_deploy_key);
		assert_eq!(
			root_count(&post_deploy_key),
			None,
			"last apply_tx output should be unrooted after post_block_update"
		);
		assert_eq!(
			root_count(&post_block_1_key),
			Some(2),
			"post-block state should be rooted at rc=2 (regular + extra persist)"
		);

		// Read-only guard 2: same but on the post-block tip (rc=2). This is the
		// state the next block's mempool validation runs against — losing it here
		// would drop history. DEPLOY now fails (contract already deployed) but the
		// invariant is still that rc is untouched on either path.
		let _ = <mock::Midnight as ValidateUnsigned>::validate_unsigned(
			TransactionSource::External,
			&deploy_call,
		);
		assert_eq!(
			root_count(&post_block_1_key),
			Some(2),
			"validate_unsigned against post-block tip must not change rc, even on error"
		);
		let _ = <mock::Midnight as ValidateUnsigned>::pre_dispatch(&deploy_call);
		assert_eq!(
			root_count(&post_block_1_key),
			Some(2),
			"pre_dispatch against post-block tip must not change rc, even on error"
		);

		// Block 2: apply STORE. The first apply of the next block unpersists the
		// previous post-block state once, dropping it from rc=2 to rc=1.
		assert_ok!(mock::Midnight::send_mn_transaction(RuntimeOrigin::none(), store_tx));
		let post_store_key = current_state_key();
		assert_eq!(
			root_count(&post_block_1_key),
			Some(1),
			"prior post-block state should drop from rc=2 to rc=1, preserved for history"
		);
		assert_eq!(root_count(&post_store_key), Some(1), "new intra-block state should be at rc=1");

		// Finalize block 2.
		process_block(3, check_ctx.clone().into());
		let post_block_2_key = current_state_key();
		assert_eq!(
			root_count(&post_store_key),
			None,
			"last apply_tx output of block 2 should be unrooted after post_block_update"
		);
		assert_eq!(
			root_count(&post_block_2_key),
			Some(2),
			"post-block-2 state should be rooted at rc=2"
		);
		assert_eq!(
			root_count(&post_block_1_key),
			Some(1),
			"prior post-block-1 state should still be alive at rc=1 for history"
		);

		// Block 3: apply CHECK and confirm the cross-block transition repeats.
		assert_ok!(mock::Midnight::send_mn_transaction(RuntimeOrigin::none(), check_tx));
		assert_eq!(
			root_count(&post_block_2_key),
			Some(1),
			"post-block-2 state drops from rc=2 to rc=1 on next block's first apply"
		);
		assert_eq!(
			root_count(&post_block_1_key),
			Some(1),
			"post-block-1 state is unaffected by block 3 — still at rc=1"
		);
		assert_eq!(
			root_count(&genesis_key),
			Some(1),
			"genesis is retained for history across the entire chain — still at rc=1"
		);
	});
}
