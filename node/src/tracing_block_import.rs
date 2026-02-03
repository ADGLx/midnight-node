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
use sc_consensus::block_import::{
	BlockCheckParams, BlockImport, BlockImportParams, ImportResult, JustificationImport,
};
use sp_runtime::{traits::{Block as BlockT, Header, SaturatedConversion}, Justification};
#[cfg(feature = "datadog-tracing")]
use opentelemetry::trace::{Tracer, Span};

/// Wraps a block importer and records a span around each `import_block` for sync diagnostics.
#[derive(Clone)]
pub struct TracingBlockImport<I>(pub I);

unsafe impl<I: Sync> Sync for TracingBlockImport<I> {}
unsafe impl<I: Send> Send for TracingBlockImport<I> {}

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
				span.set_attribute(KeyValue::new("sampled", true));
				span.set_attribute(KeyValue::new("operation", "block_validation"));
				
				let result = self.0.check_block(block).await;
				
				// Add timing information
				let duration = start_time.elapsed();
				span.set_attribute(KeyValue::new("duration_ms", duration.as_millis() as i64));
				span.end(); // Explicitly end the span
				result
			} else {
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
				
				// Start timing the overall block import process
				let start_time = std::time::Instant::now();
				let mut span = tracer.start("sync.import_block");
				span.set_attribute(KeyValue::new("block_number", format!("{}", number)));
				span.set_attribute(KeyValue::new("block_hash", format!("{:?}", block.header.hash())));
				span.set_attribute(KeyValue::new("parent_hash", format!("{:?}", block.header.parent_hash())));
				span.set_attribute(KeyValue::new("sampled", true));
				span.set_attribute(KeyValue::new("operation", "full_block_import"));
				
				let result = self.0.import_block(block).await;
				
				// Add timing breakdown attributes to the main span
				let total_duration = start_time.elapsed();
				span.set_attribute(KeyValue::new("duration_ms", total_duration.as_millis() as i64));
				span.end();
				result
			} else {
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
