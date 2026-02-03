// This file is part of midnight-node.
// Copyright (C) 2025 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// You may not use this file except in compliance with the License.
// You may obtain a copy of the License at http://www.apache.org/licenses/LICENSE-2.0
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Wraps a [`BlockImport`] to add Datadog/OpenTelemetry spans for sync diagnostics.

use async_trait::async_trait;
#[cfg(feature = "datadog-tracing")]
use parity_scale_codec::Encode;
use sc_consensus::block_import::{
	BlockCheckParams, BlockImport, BlockImportParams, ForkChoiceStrategy, ImportResult,
	JustificationImport, StateAction,
};
use sp_runtime::{traits::{Block as BlockT, Header, SaturatedConversion}, Justification};
#[cfg(feature = "datadog-tracing")]
use opentelemetry::{Context, trace::{Span, SpanBuilder, TraceContextExt, Tracer}};
#[cfg(feature = "datadog-tracing")]
use std::sync::atomic::{AtomicU64, Ordering};
#[cfg(feature = "datadog-tracing")]
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Wraps a block importer and records a span around each `import_block` for sync diagnostics.
#[derive(Clone)]
pub struct TracingBlockImport<I>(pub I);

unsafe impl<I: Sync> Sync for TracingBlockImport<I> {}
unsafe impl<I: Send> Send for TracingBlockImport<I> {}

#[cfg(feature = "datadog-tracing")]
fn state_action_label<Block: BlockT>(action: &StateAction<Block>) -> &'static str {
	match action {
		StateAction::ApplyChanges(_) => "apply_changes",
		StateAction::Execute => "execute",
		StateAction::ExecuteIfPossible => "execute_if_possible",
		StateAction::Skip => "skip",
	}
}

#[cfg(feature = "datadog-tracing")]
fn now_ns() -> u64 {
	SystemTime::now()
		.duration_since(UNIX_EPOCH)
		.unwrap_or(Duration::from_secs(0))
		.as_nanos() as u64
}

#[cfg(feature = "datadog-tracing")]
static LAST_IMPORT_END_NS: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "datadog-tracing")]
static LAST_CHECK_END_NS: AtomicU64 = AtomicU64::new(0);

#[async_trait]
impl<B, I> BlockImport<B> for TracingBlockImport<I>
where
	B: BlockT + Send,
	B::Header: Send,
	I: BlockImport<B> + Send + Sync,
	I::Error: Send + 'static,
{
	type Error = I::Error;

	async fn check_block(
		&self,
		block: BlockCheckParams<B>,
	) -> Result<ImportResult, Self::Error> {
		#[cfg(feature = "datadog-tracing")]
		{
			let number = block.number;
			
			// Sample blocks for check_block tracing
			let sample_rate = std::env::var("DD_TRACE_SAMPLE_RATE")
				.ok()
				.and_then(|s| s.parse::<u64>().ok())
				.unwrap_or(100);
			
			if number.saturated_into::<u64>() % sample_rate == 0 {
				use opentelemetry::KeyValue;
				
				let tracer = opentelemetry::global::tracer("midnight-node");
				
				// Start timing the block validation process  
				let start_time = std::time::Instant::now();
				let mut span = tracer.start("sync.check_block");
				span.set_attribute(KeyValue::new("block_number", format!("{}", number)));
				span.set_attribute(KeyValue::new("block_hash", format!("{:?}", block.hash)));
				span.set_attribute(KeyValue::new("parent_hash", format!("{:?}", block.parent_hash)));
				span.set_attribute(KeyValue::new("allow_missing_state", block.allow_missing_state));
				span.set_attribute(KeyValue::new("allow_missing_parent", block.allow_missing_parent));
				span.set_attribute(KeyValue::new("import_existing", block.import_existing));
				span.set_attribute(KeyValue::new("sampled", true));
				span.set_attribute(KeyValue::new("operation", "block_validation"));
				
				let result = self.0.check_block(block).await;
				
				// Add timing information
				let duration = start_time.elapsed();
				span.set_attribute(KeyValue::new("duration_ms", duration.as_millis() as i64));
				span.end(); // Explicitly end the span
				LAST_CHECK_END_NS.store(now_ns(), Ordering::Relaxed);
				result
			} else {
				LAST_CHECK_END_NS.store(now_ns(), Ordering::Relaxed);
				self.0.check_block(block).await
			}
		}
		#[cfg(not(feature = "datadog-tracing"))]
		{
			self.0.check_block(block).await
		}
	}

	async fn import_block(
		&self,
		block: BlockImportParams<B>,
	) -> Result<ImportResult, Self::Error> {
		#[cfg(feature = "datadog-tracing")]
		{
			let number = *block.header.number();
			
			// Sample blocks based on DD_TRACE_SAMPLE_RATE (default 1% = 100, 5% = 20, 10% = 10)
			let sample_rate = std::env::var("DD_TRACE_SAMPLE_RATE")
				.ok()
				.and_then(|s| s.parse::<u64>().ok())
				.unwrap_or(100); // Default: sample 1% (every 100th block)
			
			if number.saturated_into::<u64>() % sample_rate == 0 {
				use opentelemetry::KeyValue;
				
				let tracer = opentelemetry::global::tracer("midnight-node");
				
				let start_ns = now_ns();
				let last_import_end_ns = LAST_IMPORT_END_NS.load(Ordering::Relaxed);
				let last_check_end_ns = LAST_CHECK_END_NS.load(Ordering::Relaxed);

				let body_len = block.body.as_ref().map(|body| body.len()).unwrap_or(0);
				let indexed_body_len = block
					.indexed_body
					.as_ref()
					.map(|body| body.len())
					.unwrap_or(0);
				let body_bytes = block
					.body
					.as_ref()
					.map(|body| body.iter().map(|ext| ext.encode().len() as i64).sum())
					.unwrap_or(0);
				let indexed_body_bytes = block
					.indexed_body
					.as_ref()
					.map(|body| body.iter().map(|ext| ext.len() as i64).sum())
					.unwrap_or(0);
				let state_action = state_action_label(&block.state_action);

				// Start timing the overall block import process
				let start_time = std::time::Instant::now();
				let mut parent_span = tracer.start("sync.import_block");
				parent_span.set_attribute(KeyValue::new("block_number", format!("{}", number)));
				parent_span.set_attribute(KeyValue::new("block_hash", format!("{:?}", block.header.hash())));
				parent_span.set_attribute(KeyValue::new("parent_hash", format!("{:?}", block.header.parent_hash())));
				parent_span.set_attribute(KeyValue::new("origin", format!("{:?}", block.origin)));
				parent_span.set_attribute(KeyValue::new("finalized", block.finalized));
				parent_span.set_attribute(KeyValue::new("import_existing", block.import_existing));
				parent_span.set_attribute(KeyValue::new("create_gap", block.create_gap));
				parent_span.set_attribute(KeyValue::new("has_justifications", block.justifications.is_some()));
				parent_span.set_attribute(KeyValue::new("post_digests_len", block.post_digests.len() as i64));
				parent_span.set_attribute(KeyValue::new("auxiliary_len", block.auxiliary.len() as i64));
				parent_span.set_attribute(KeyValue::new("intermediates_len", block.intermediates.len() as i64));
				parent_span.set_attribute(KeyValue::new("body_len", body_len as i64));
				parent_span.set_attribute(KeyValue::new("indexed_body_len", indexed_body_len as i64));
				parent_span.set_attribute(KeyValue::new("body_bytes", body_bytes));
				parent_span.set_attribute(KeyValue::new("indexed_body_bytes", indexed_body_bytes));
				parent_span.set_attribute(KeyValue::new("state_action", state_action));
				parent_span.set_attribute(KeyValue::new("fork_choice", match block.fork_choice {
					Some(ForkChoiceStrategy::LongestChain) => "longest",
					Some(ForkChoiceStrategy::Custom(true)) => "custom_true",
					Some(ForkChoiceStrategy::Custom(false)) => "custom_false",
					None => "none",
				}));
				parent_span.set_attribute(KeyValue::new("sampled", true));
				parent_span.set_attribute(KeyValue::new("operation", "full_block_import"));

				let cx = Context::current_with_span(parent_span);

				// Span: idle gap since previous import finished (approx queue/download wait)
				if last_import_end_ns > 0 && start_ns > last_import_end_ns {
					let gap_ns = start_ns - last_import_end_ns;
					let gap_start = UNIX_EPOCH + Duration::from_nanos(last_import_end_ns);
					let gap_end = UNIX_EPOCH + Duration::from_nanos(start_ns);
					let mut gap_span = tracer.build_with_context(
						SpanBuilder {
							name: "sync.import_gap".into(),
							start_time: Some(gap_start),
							end_time: Some(gap_end),
							attributes: Some(vec![
								KeyValue::new("gap_ms", (gap_ns / 1_000_000) as i64),
								KeyValue::new("gap_type", "idle_or_download_wait"),
							]),
							..Default::default()
						},
						&cx,
					);
					gap_span.end();
				}

				// Span: time from last check_block completion to this import (pipeline delay)
				if last_check_end_ns > 0 && start_ns > last_check_end_ns {
					let delay_ns = start_ns - last_check_end_ns;
					let delay_start = UNIX_EPOCH + Duration::from_nanos(last_check_end_ns);
					let delay_end = UNIX_EPOCH + Duration::from_nanos(start_ns);
					let mut delay_span = tracer.build_with_context(
						SpanBuilder {
							name: "sync.check_to_import_gap".into(),
							start_time: Some(delay_start),
							end_time: Some(delay_end),
							attributes: Some(vec![
								KeyValue::new("delay_ms", (delay_ns / 1_000_000) as i64),
								KeyValue::new("gap_type", "pipeline_wait"),
							]),
							..Default::default()
						},
						&cx,
					);
					delay_span.end();
				}
				let mut child_span = tracer.start_with_context("sync.import_block.exec", &cx);
				child_span.set_attribute(KeyValue::new("phase", "block_processing"));

				let result = self.0.import_block(block).await;

				child_span.end();

				// Add timing breakdown attributes to the main span
				let total_duration = start_time.elapsed();
				let parent_span = cx.span();
				parent_span.set_attribute(KeyValue::new("duration_ms", total_duration.as_millis() as i64));
				parent_span.end();
				LAST_IMPORT_END_NS.store(now_ns(), Ordering::Relaxed);
				result
			} else {
				LAST_IMPORT_END_NS.store(now_ns(), Ordering::Relaxed);
				self.0.import_block(block).await
			}
		}
		#[cfg(not(feature = "datadog-tracing"))]
		{
			self.0.import_block(block).await
		}
	}
}

#[async_trait]
impl<B, I> JustificationImport<B> for TracingBlockImport<I>
where
	B: BlockT + Send,
	I: JustificationImport<B> + Send + Sync,
	I::Error: Send + 'static,
{
	type Error = I::Error;

	async fn on_start(&mut self) -> Vec<(B::Hash, <<B as BlockT>::Header as Header>::Number)> {
		self.0.on_start().await
	}

	async fn import_justification(
		&mut self,
		hash: B::Hash,
		number: <<B as BlockT>::Header as Header>::Number,
		justification: Justification,
	) -> Result<(), <Self as JustificationImport<B>>::Error> {
		self.0.import_justification(hash, number, justification).await
	}
}
