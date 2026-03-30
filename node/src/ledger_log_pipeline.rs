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

use crate::service::FullClient;
use futures::StreamExt;
use log::{debug, error, trace};
use sc_client_api::BlockchainEvents;
use sc_service::TaskManager;
use std::{
	panic::{self, AssertUnwindSafe},
	sync::{
		Arc,
		atomic::{AtomicBool, Ordering},
	},
	time::Instant,
};
use tokio::sync::Notify;

const LOG_TARGET: &str = "ledger-parity-db-wal";

pub(crate) fn spawn(task_manager: &TaskManager, client: Arc<FullClient>) {
	let flush_requested = Arc::new(AtomicBool::new(false));
	let flush_notify = Arc::new(Notify::new());

	task_manager.spawn_handle().spawn("ledger-parity-db-wal-imports", None, {
		let flush_requested = flush_requested.clone();
		let flush_notify = flush_notify.clone();

		async move {
			let mut import_notifications = client.every_import_notification_stream();

			while let Some(notification) = import_notifications.next().await {
				flush_requested.store(true, Ordering::Release);
				flush_notify.notify_one();
				trace!(
					target: LOG_TARGET,
					"queued WAL flush: import {:?} origin {:?}",
					notification.hash,
					notification.origin,
				);
			}
		}
	});

	task_manager.spawn_handle().spawn("ledger-parity-db-wal-flush", None, {
		let flush_requested = flush_requested.clone();
		let flush_notify = flush_notify.clone();

		async move {
			loop {
				flush_notify.notified().await;

				while flush_requested.swap(false, Ordering::AcqRel) {
					debug!(target: LOG_TARGET, "WAL flush start");
					let started_at = Instant::now();
					let result = tokio::task::spawn_blocking(|| {
						panic::catch_unwind(AssertUnwindSafe(|| {
							midnight_node_ledger::flush_log_pipeline_on_default_storage();
						}))
					})
					.await;

					match result {
						Ok(Ok(())) => debug!(
							target: LOG_TARGET,
							"WAL flush done in {} ms",
							started_at.elapsed().as_millis()
						),
						Ok(Err(_)) => error!(
							target: LOG_TARGET,
							"WAL flush panicked; will retry on next import"
						),
						Err(error) => error!(
							target: LOG_TARGET,
							"WAL flush task join failed: {error}"
						),
					}
				}
			}
		}
	});
}
