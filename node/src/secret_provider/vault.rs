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

//! HashiCorp Vault secret provider
//!
//! URI format: `vault://secret/data/path#field` or `vault://secret/data/path?field=key`
//!
//! Examples:
//! - `vault://secret/data/midnight/aura#seed` - KV v2 secret with field
//! - `vault://secret/midnight/aura#seed` - KV v1 secret with field
//! - `vault://secret/data/midnight/seeds?field=aura`
//!
//! Authentication is handled via environment variables:
//! - VAULT_ADDR: Vault server address (required)
//! - VAULT_TOKEN: Token authentication
//! - VAULT_ROLE_ID + VAULT_SECRET_ID: AppRole authentication
//! - VAULT_KUBERNETES_ROLE: Kubernetes authentication
//!
//! Additional options:
//! - VAULT_NAMESPACE: Vault namespace (for enterprise)
//! - VAULT_SKIP_VERIFY: Skip TLS verification (not recommended)

use std::collections::HashMap;

use super::{SecretProvider, SecretProviderError};

/// Provider for HashiCorp Vault
#[allow(dead_code)] // Fields are used when `vault-secrets` feature is enabled
pub struct VaultSecretProvider {
	/// Secret path (e.g., "secret/data/my-app")
	path: String,
	/// Field within the secret to extract
	field: Option<String>,
	/// Vault version (for KV v2 secrets)
	version: Option<u32>,
}

impl VaultSecretProvider {
	pub fn new(path: String, field: Option<String>, params: HashMap<String, String>) -> Self {
		let version = params.get("version").and_then(|v| v.parse().ok());
		let field = field.or_else(|| params.get("field").cloned());

		Self { path, field, version }
	}
}

#[async_trait::async_trait]
impl SecretProvider for VaultSecretProvider {
	async fn get_secret(&self) -> Result<String, SecretProviderError> {
		#[cfg(feature = "vault-secrets")]
		{
			use vaultrs::client::{VaultClient, VaultClientSettingsBuilder};
			use vaultrs::kv2;

			// Get Vault address
			let vault_addr = std::env::var("VAULT_ADDR").map_err(|_| {
				SecretProviderError::VaultError(
					"VAULT_ADDR environment variable not set".to_string(),
				)
			})?;

			// Build client settings
			let mut settings_builder = VaultClientSettingsBuilder::default();
			settings_builder.address(&vault_addr);

			if let Ok(namespace) = std::env::var("VAULT_NAMESPACE") {
				settings_builder.namespace(Some(namespace));
			}

			if std::env::var("VAULT_SKIP_VERIFY").is_ok() {
				settings_builder.verify(false);
			}

			// Get token (directly or via other auth methods)
			let token = get_vault_token().await?;
			settings_builder.token(&token);

			let settings = settings_builder.build().map_err(|e| {
				SecretProviderError::VaultError(format!("Failed to build Vault settings: {}", e))
			})?;

			let client = VaultClient::new(settings).map_err(|e| {
				SecretProviderError::VaultError(format!("Failed to create Vault client: {}", e))
			})?;

			// Parse the path to extract mount point and secret path
			// e.g., "secret/data/my-app" -> mount="secret", path="my-app"
			let (mount, secret_path) = parse_vault_path(&self.path)?;

			// Read the secret
			let secret: HashMap<String, serde_json::Value> =
				kv2::read(&client, &mount, &secret_path).await.map_err(|e| {
					SecretProviderError::VaultError(format!(
						"Failed to read secret '{}': {}",
						self.path, e
					))
				})?;

			// Extract the field
			if let Some(field) = &self.field {
				secret
					.get(field)
					.and_then(|v| v.as_str())
					.map(|s| s.trim().to_string())
					.ok_or_else(|| {
						SecretProviderError::VaultError(format!(
							"Field '{}' not found in secret or is not a string",
							field
						))
					})
			} else {
				// If no field specified, try to get a single value or return error
				if secret.len() == 1 {
					secret
						.values()
						.next()
						.and_then(|v| v.as_str())
						.map(|s| s.trim().to_string())
						.ok_or_else(|| {
							SecretProviderError::VaultError(
								"Secret value is not a string".to_string(),
							)
						})
				} else {
					Err(SecretProviderError::VaultError(format!(
						"Secret has multiple fields ({}), please specify which field to use with #field or ?field=name",
						secret.keys().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")
					)))
				}
			}
		}

		#[cfg(not(feature = "vault-secrets"))]
		{
			Err(SecretProviderError::VaultError(
				"HashiCorp Vault support not compiled in. \
				Enable the 'vault-secrets' feature to use vault:// URIs."
					.to_string(),
			))
		}
	}
}

#[cfg(feature = "vault-secrets")]
async fn get_vault_token() -> Result<String, SecretProviderError> {
	// Try direct token first
	if let Ok(token) = std::env::var("VAULT_TOKEN") {
		return Ok(token);
	}

	// Try AppRole authentication
	if let (Ok(role_id), Ok(secret_id)) =
		(std::env::var("VAULT_ROLE_ID"), std::env::var("VAULT_SECRET_ID"))
	{
		return authenticate_approle(&role_id, &secret_id).await;
	}

	// Try Kubernetes authentication
	if let Ok(role) = std::env::var("VAULT_KUBERNETES_ROLE") {
		return authenticate_kubernetes(&role).await;
	}

	Err(SecretProviderError::VaultError(
		"No Vault authentication method available. Set VAULT_TOKEN, \
		or VAULT_ROLE_ID + VAULT_SECRET_ID, or VAULT_KUBERNETES_ROLE"
			.to_string(),
	))
}

#[cfg(feature = "vault-secrets")]
async fn authenticate_approle(
	role_id: &str,
	secret_id: &str,
) -> Result<String, SecretProviderError> {
	use vaultrs::auth::approle;
	use vaultrs::client::{VaultClient, VaultClientSettingsBuilder};

	let vault_addr = std::env::var("VAULT_ADDR").map_err(|_| {
		SecretProviderError::VaultError("VAULT_ADDR environment variable not set".to_string())
	})?;

	let mut settings_builder = VaultClientSettingsBuilder::default();
	settings_builder.address(&vault_addr);

	let settings = settings_builder.build().map_err(|e| {
		SecretProviderError::VaultError(format!("Failed to build Vault settings: {}", e))
	})?;

	let client = VaultClient::new(settings).map_err(|e| {
		SecretProviderError::VaultError(format!("Failed to create Vault client: {}", e))
	})?;

	let auth_info = approle::login(&client, "approle", role_id, secret_id).await.map_err(|e| {
		SecretProviderError::VaultError(format!("AppRole authentication failed: {}", e))
	})?;

	Ok(auth_info.client_token)
}

#[cfg(feature = "vault-secrets")]
async fn authenticate_kubernetes(role: &str) -> Result<String, SecretProviderError> {
	use vaultrs::auth::kubernetes;
	use vaultrs::client::{VaultClient, VaultClientSettingsBuilder};

	let vault_addr = std::env::var("VAULT_ADDR").map_err(|_| {
		SecretProviderError::VaultError("VAULT_ADDR environment variable not set".to_string())
	})?;

	// Read the service account token
	let jwt = tokio::fs::read_to_string("/var/run/secrets/kubernetes.io/serviceaccount/token")
		.await
		.map_err(|e| {
			SecretProviderError::VaultError(format!(
				"Failed to read Kubernetes service account token: {}",
				e
			))
		})?;

	let mut settings_builder = VaultClientSettingsBuilder::default();
	settings_builder.address(&vault_addr);

	let settings = settings_builder.build().map_err(|e| {
		SecretProviderError::VaultError(format!("Failed to build Vault settings: {}", e))
	})?;

	let client = VaultClient::new(settings).map_err(|e| {
		SecretProviderError::VaultError(format!("Failed to create Vault client: {}", e))
	})?;

	let auth_info = kubernetes::login(&client, "kubernetes", role, &jwt).await.map_err(|e| {
		SecretProviderError::VaultError(format!("Kubernetes authentication failed: {}", e))
	})?;

	Ok(auth_info.client_token)
}

#[cfg(feature = "vault-secrets")]
fn parse_vault_path(path: &str) -> Result<(String, String), SecretProviderError> {
	// Handle paths like "secret/data/my-app" -> mount="secret", path="my-app"
	// Or "secret/my-app" for KV v1 -> mount="secret", path="my-app"

	let parts: Vec<&str> = path.splitn(3, '/').collect();

	if parts.len() < 2 {
		return Err(SecretProviderError::VaultError(format!(
			"Invalid Vault path '{}'. Expected format: mount/path or mount/data/path",
			path
		)));
	}

	let mount = parts[0].to_string();

	// Check if this is a KV v2 path (contains "data")
	let secret_path = if parts.len() == 3 && parts[1] == "data" {
		parts[2].to_string()
	} else if parts.len() == 2 {
		parts[1].to_string()
	} else {
		// Reconstruct path after mount
		parts[1..].join("/")
	};

	Ok((mount, secret_path))
}
