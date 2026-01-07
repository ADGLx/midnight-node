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

//! AWS Secrets Manager provider
//!
//! URI format: `aws://secret-name` or `aws://secret-name?region=us-east-1&version_id=xxx`
//!
//! Authentication is handled via the standard AWS credential chain:
//! - Environment variables (AWS_ACCESS_KEY_ID, AWS_SECRET_ACCESS_KEY)
//! - Shared credentials file (~/.aws/credentials)
//! - IAM role (when running on EC2/ECS/Lambda)
//!
//! Environment variables:
//! - AWS_REGION or region parameter in URI
//! - AWS_SECRET_KEY_FIELD: JSON field to extract (if secret is JSON)

use std::collections::HashMap;

use super::{SecretProvider, SecretProviderError};

/// Provider for AWS Secrets Manager
#[allow(dead_code)] // Fields are used when `aws-secrets` feature is enabled
pub struct AwsSecretsProvider {
	secret_name: String,
	region: Option<String>,
	version_id: Option<String>,
	version_stage: Option<String>,
	/// JSON field to extract from secret (for secrets stored as JSON objects)
	key_field: Option<String>,
}

impl AwsSecretsProvider {
	pub fn new(secret_name: String, params: HashMap<String, String>) -> Self {
		Self {
			secret_name,
			region: params.get("region").cloned(),
			version_id: params.get("version_id").cloned(),
			version_stage: params.get("version_stage").cloned(),
			key_field: params
				.get("field")
				.cloned()
				.or_else(|| std::env::var("AWS_SECRET_KEY_FIELD").ok()),
		}
	}
}

#[async_trait::async_trait]
impl SecretProvider for AwsSecretsProvider {
	async fn get_secret(&self) -> Result<String, SecretProviderError> {
		#[cfg(feature = "aws-secrets")]
		{
			use aws_config::BehaviorVersion;
			use aws_sdk_secretsmanager::Client;

			// Load AWS config
			let mut config_loader = aws_config::defaults(BehaviorVersion::latest());
			if let Some(region) = &self.region {
				config_loader = config_loader.region(aws_config::Region::new(region.clone()));
			}
			let config = config_loader.load().await;
			let client = Client::new(&config);

			// Build the request
			let mut request = client.get_secret_value().secret_id(&self.secret_name);

			if let Some(version_id) = &self.version_id {
				request = request.version_id(version_id);
			}
			if let Some(version_stage) = &self.version_stage {
				request = request.version_stage(version_stage);
			}

			// Execute the request
			let response = request.send().await.map_err(|e| {
				SecretProviderError::AwsError(format!(
					"Failed to retrieve secret '{}': {}",
					self.secret_name, e
				))
			})?;

			// Get the secret string
			let secret_string = response
				.secret_string()
				.ok_or_else(|| {
					SecretProviderError::AwsError(format!(
						"Secret '{}' does not contain a string value (binary secrets not supported)",
						self.secret_name
					))
				})?
				.to_string();

			// If a key field is specified, parse as JSON and extract the field
			if let Some(field) = &self.key_field {
				let json: serde_json::Value =
					serde_json::from_str(&secret_string).map_err(|e| {
						SecretProviderError::AwsError(format!(
							"Failed to parse secret as JSON for field extraction: {}",
							e
						))
					})?;

				json.get(field)
					.and_then(|v| v.as_str())
					.map(|s| s.trim().to_string())
					.ok_or_else(|| {
						SecretProviderError::AwsError(format!(
							"Field '{}' not found in secret or is not a string",
							field
						))
					})
			} else {
				Ok(secret_string.trim().to_string())
			}
		}

		#[cfg(not(feature = "aws-secrets"))]
		{
			Err(SecretProviderError::AwsError(
				"AWS Secrets Manager support not compiled in. \
				Enable the 'aws-secrets' feature to use aws:// URIs."
					.to_string(),
			))
		}
	}
}
