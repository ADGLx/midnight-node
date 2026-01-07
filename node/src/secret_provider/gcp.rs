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

//! GCP Secret Manager provider
//!
//! URI format: `gcp://projects/PROJECT_ID/secrets/SECRET_NAME/versions/VERSION`
//!
//! Examples:
//! - `gcp://projects/my-project/secrets/aura-seed/versions/latest`
//! - `gcp://projects/my-project/secrets/aura-seed/versions/1`
//!
//! Authentication is handled via Application Default Credentials (ADC):
//! - GOOGLE_APPLICATION_CREDENTIALS environment variable pointing to service account key
//! - Metadata server (when running on GCP)
//! - gcloud CLI credentials

use std::collections::HashMap;

use super::{SecretProvider, SecretProviderError};

/// Provider for GCP Secret Manager
#[allow(dead_code)] // Fields are used when `gcp-secrets` feature is enabled
pub struct GcpSecretProvider {
	/// Full secret path: projects/PROJECT/secrets/SECRET/versions/VERSION
	secret_path: String,
	/// JSON field to extract from secret (if secret is JSON)
	key_field: Option<String>,
}

impl GcpSecretProvider {
	pub fn new(secret_path: String, params: HashMap<String, String>) -> Self {
		Self {
			secret_path,
			key_field: params
				.get("field")
				.cloned()
				.or_else(|| std::env::var("GCP_SECRET_KEY_FIELD").ok()),
		}
	}
}

#[async_trait::async_trait]
impl SecretProvider for GcpSecretProvider {
	async fn get_secret(&self) -> Result<String, SecretProviderError> {
		#[cfg(feature = "gcp-secrets")]
		{
			use google_cloud_secretmanager_v1::client::SecretManagerService;

			// Create client with default credentials (uses Application Default Credentials)
			let client = SecretManagerService::builder().build().await.map_err(|e| {
				SecretProviderError::GcpError(format!("Failed to create client: {}", e))
			})?;

			// Access the secret version
			let response = client
				.access_secret_version()
				.set_name(&self.secret_path)
				.send()
				.await
				.map_err(|e| {
					SecretProviderError::GcpError(format!(
						"Failed to access secret '{}': {}",
						self.secret_path, e
					))
				})?;

			// Get the payload
			let payload = response.payload.ok_or_else(|| {
				SecretProviderError::GcpError("Secret has no payload".to_string())
			})?;

			let secret_string = String::from_utf8(payload.data.to_vec()).map_err(|e| {
				SecretProviderError::GcpError(format!("Secret is not valid UTF-8: {}", e))
			})?;

			// If a key field is specified, parse as JSON and extract the field
			if let Some(field) = &self.key_field {
				let json: serde_json::Value =
					serde_json::from_str(&secret_string).map_err(|e| {
						SecretProviderError::GcpError(format!(
							"Failed to parse secret as JSON for field extraction: {}",
							e
						))
					})?;

				json.get(field)
					.and_then(|v| v.as_str())
					.map(|s| s.trim().to_string())
					.ok_or_else(|| {
						SecretProviderError::GcpError(format!(
							"Field '{}' not found in secret or is not a string",
							field
						))
					})
			} else {
				Ok(secret_string.trim().to_string())
			}
		}

		#[cfg(not(feature = "gcp-secrets"))]
		{
			Err(SecretProviderError::GcpError(
				"GCP Secret Manager support not compiled in. \
				Enable the 'gcp-secrets' feature to use gcp:// URIs."
					.to_string(),
			))
		}
	}
}
