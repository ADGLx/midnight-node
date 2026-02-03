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

//! OpenTelemetry trace handler for bridging Substrate spans to Datadog.
//!
//! This module provides a [`TraceHandler`] implementation that converts Substrate's
//! native tracing spans into OpenTelemetry spans and exports them to Datadog.

use opentelemetry::{
	Context, KeyValue, global,
	trace::{SpanBuilder, SpanKind, TraceContextExt},
};
use sc_tracing::{SpanDatum, TraceEvent, TraceHandler};
use std::{
	collections::HashMap,
	sync::Mutex,
	time::{Duration, SystemTime},
};

/// A trace handler that exports Substrate spans to OpenTelemetry/Datadog.
///
/// This handler receives completed span data from Substrate's tracing system
/// and converts them into OpenTelemetry spans with proper timing and attributes.
pub struct OpenTelemetryTraceHandler {
	service_name: String,
	/// Maps Substrate span IDs to OpenTelemetry contexts for parent-child relationships
	span_contexts: Mutex<HashMap<u64, Context>>,
}

impl OpenTelemetryTraceHandler {
	/// Creates a new OpenTelemetry trace handler.
	///
	/// # Arguments
	/// * `service_name` - The service name to use for spans (e.g., "midnight-node")
	pub fn new(service_name: &str) -> Self {
		Self { service_name: service_name.to_string(), span_contexts: Mutex::new(HashMap::new()) }
	}

	/// Converts a tracing Level to an OpenTelemetry-compatible string.
	fn level_to_string(level: &tracing::Level) -> &'static str {
		match *level {
			tracing::Level::ERROR => "ERROR",
			tracing::Level::WARN => "WARN",
			tracing::Level::INFO => "INFO",
			tracing::Level::DEBUG => "DEBUG",
			tracing::Level::TRACE => "TRACE",
		}
	}

	/// Converts Substrate span values to OpenTelemetry attributes.
	fn values_to_attributes(values: &sc_tracing::Values) -> Vec<KeyValue> {
		let mut attrs = Vec::new();

		for (k, v) in &values.bool_values {
			attrs.push(KeyValue::new(k.clone(), *v));
		}
		for (k, v) in &values.i64_values {
			attrs.push(KeyValue::new(k.clone(), *v));
		}
		for (k, v) in &values.u64_values {
			attrs.push(KeyValue::new(k.clone(), *v as i64));
		}
		for (k, v) in &values.string_values {
			attrs.push(KeyValue::new(k.clone(), v.clone()));
		}

		attrs
	}

	/// Calculates the start time from overall duration.
	/// Since we receive completed spans, we need to work backwards from now.
	fn calculate_start_time(overall_time: Duration) -> SystemTime {
		SystemTime::now().checked_sub(overall_time).unwrap_or_else(SystemTime::now)
	}
}

impl TraceHandler for OpenTelemetryTraceHandler {
	fn handle_span(&self, span_datum: &SpanDatum) {
		let tracer = global::tracer(self.service_name.clone());

		// Build attributes from span data
		let mut attributes = Self::values_to_attributes(&span_datum.values);
		attributes.push(KeyValue::new("substrate.target", span_datum.target.clone()));
		attributes.push(KeyValue::new("substrate.level", Self::level_to_string(&span_datum.level)));
		attributes.push(KeyValue::new("substrate.line", span_datum.line as i64));
		attributes.push(KeyValue::new("substrate.span_id", span_datum.id.into_u64() as i64));

		// Calculate timing
		let start_time = Self::calculate_start_time(span_datum.overall_time);
		let end_time = SystemTime::now();

		// Get parent context if available
		let parent_context = span_datum.parent_id.as_ref().and_then(|parent_id| {
			self.span_contexts
				.lock()
				.ok()
				.and_then(|contexts| contexts.get(&parent_id.into_u64()).cloned())
		});

		// Create the span builder
		let span_builder = SpanBuilder::from_name(span_datum.name.clone())
			.with_kind(SpanKind::Internal)
			.with_start_time(start_time)
			.with_attributes(attributes);

		// Build and start the span with proper parent context
		let span = if let Some(ref parent_ctx) = parent_context {
			span_builder.start_with_context(&tracer, parent_ctx)
		} else {
			span_builder.start(&tracer)
		};

		// Create context for potential child spans
		let cx = Context::current_with_span(span);

		// Store the context for child spans to reference
		if let Ok(mut contexts) = self.span_contexts.lock() {
			contexts.insert(span_datum.id.into_u64(), cx.clone());

			// Clean up old contexts to prevent memory leaks
			// Keep only recent contexts (parent might still be referenced)
			if contexts.len() > 10000 {
				// Simple cleanup: remove oldest entries
				// In production, you might want a more sophisticated LRU cache
				let keys_to_remove: Vec<_> =
					contexts.keys().take(contexts.len() / 2).cloned().collect();
				for key in keys_to_remove {
					contexts.remove(&key);
				}
			}
		}

		// End the span with the calculated end time
		// The span is automatically ended when dropped, but we want explicit timing
		cx.span().end_with_timestamp(end_time);
	}

	fn handle_event(&self, event: &TraceEvent) {
		let tracer = global::tracer(self.service_name.clone());

		// Convert event to a short-lived span (events don't have duration)
		let mut attributes = Self::values_to_attributes(&event.values);
		attributes.push(KeyValue::new("substrate.target", event.target.clone()));
		attributes.push(KeyValue::new("substrate.level", Self::level_to_string(&event.level)));
		attributes.push(KeyValue::new("event.name", event.name.clone()));

		// Get parent context if available
		let parent_context = event.parent_id.as_ref().and_then(|parent_id| {
			self.span_contexts
				.lock()
				.ok()
				.and_then(|contexts| contexts.get(&parent_id.into_u64()).cloned())
		});

		let now = SystemTime::now();
		let span_builder = SpanBuilder::from_name(format!("event: {}", event.name))
			.with_kind(SpanKind::Internal)
			.with_start_time(now)
			.with_attributes(attributes);

		let span = if let Some(ref parent_ctx) = parent_context {
			span_builder.start_with_context(&tracer, parent_ctx)
		} else {
			span_builder.start(&tracer)
		};

		// Events are instantaneous, so end immediately
		let cx = Context::current_with_span(span);
		cx.span().end_with_timestamp(now);
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_level_to_string() {
		assert_eq!(OpenTelemetryTraceHandler::level_to_string(&tracing::Level::ERROR), "ERROR");
		assert_eq!(OpenTelemetryTraceHandler::level_to_string(&tracing::Level::INFO), "INFO");
	}

	#[test]
	fn test_calculate_start_time() {
		let duration = Duration::from_secs(1);
		let start = OpenTelemetryTraceHandler::calculate_start_time(duration);
		let now = SystemTime::now();

		// Start time should be approximately 1 second before now
		let elapsed = now.duration_since(start).unwrap();
		assert!(elapsed >= Duration::from_millis(900));
		assert!(elapsed <= Duration::from_millis(1100));
	}

	#[test]
	fn test_handler_creation() {
		let handler = OpenTelemetryTraceHandler::new("test-service");
		assert_eq!(handler.service_name, "test-service");
	}
}
