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

//! Secret provider abstraction for loading seeds from various sources.
//!
//! Supported URI schemes:
//! - `file:///path/to/file` or just `/path/to/file` - Local file
//! - `aws://secret-name` or `aws://secret-name?region=us-east-1&version_id=xxx`
//! - `gcp://projects/PROJECT/secrets/SECRET/versions/VERSION`
//! - `vault://secret/path#field` or `vault://secret/path?field=key`
//!
//! For Vault, authentication is handled via environment variables:
//! - VAULT_ADDR: Vault server address
//! - VAULT_TOKEN: Vault token (for token auth)
//! - Or role-based auth via VAULT_ROLE_ID and VAULT_SECRET_ID

use std::collections::HashMap;

mod aws;
mod file;
mod gcp;
mod vault;

pub use aws::AwsSecretsProvider;
pub use file::FileSecretProvider;
pub use gcp::GcpSecretProvider;
pub use vault::VaultSecretProvider;

/// Error type for secret provider operations
#[derive(Debug, thiserror::Error)]
pub enum SecretProviderError {
	#[error("Unsupported secret URI scheme: {0}")]
	UnsupportedScheme(String),

	#[error("Invalid secret URI: {0}")]
	InvalidUri(String),

	#[error("Failed to read secret from file: {0}")]
	FileError(#[from] std::io::Error),

	#[error("AWS Secrets Manager error: {0}")]
	AwsError(String),

	#[error("GCP Secret Manager error: {0}")]
	GcpError(String),

	#[error("HashiCorp Vault error: {0}")]
	VaultError(String),

	#[error("Secret not found: {0}")]
	NotFound(String),

	#[error("Configuration error: {0}")]
	ConfigError(String),
}

/// Trait for secret providers
#[async_trait::async_trait]
pub trait SecretProvider: Send + Sync {
	/// Fetch the secret value as a string
	async fn get_secret(&self) -> Result<String, SecretProviderError>;
}

/// Parse a secret URI and return the appropriate provider
pub fn parse_secret_uri(uri: &str) -> Result<Box<dyn SecretProvider>, SecretProviderError> {
	// Handle plain file paths (backward compatibility)
	if !uri.contains("://") {
		return Ok(Box::new(FileSecretProvider::new(uri.to_string())));
	}

	let (scheme, rest) = uri
		.split_once("://")
		.ok_or_else(|| SecretProviderError::InvalidUri(uri.to_string()))?;

	match scheme {
		"file" => {
			// file:///path/to/file -> /path/to/file
			let path = rest.to_string();
			Ok(Box::new(FileSecretProvider::new(path)))
		},
		"aws" => {
			let (secret_name, params) = parse_uri_with_params(rest)?;
			Ok(Box::new(AwsSecretsProvider::new(secret_name, params)))
		},
		"gcp" => {
			let (secret_path, params) = parse_uri_with_params(rest)?;
			Ok(Box::new(GcpSecretProvider::new(secret_path, params)))
		},
		"vault" => {
			let (path, params) = parse_uri_with_params(rest)?;
			// Handle fragment for field selection (vault://path#field)
			let (path, field) = if let Some((p, f)) = path.split_once('#') {
				(p.to_string(), Some(f.to_string()))
			} else {
				(path, params.get("field").cloned())
			};
			Ok(Box::new(VaultSecretProvider::new(path, field, params)))
		},
		_ => Err(SecretProviderError::UnsupportedScheme(scheme.to_string())),
	}
}

/// Parse URI path and query parameters
fn parse_uri_with_params(
	rest: &str,
) -> Result<(String, HashMap<String, String>), SecretProviderError> {
	let (path, query) = rest.split_once('?').unwrap_or((rest, ""));

	let params: HashMap<String, String> = query
		.split('&')
		.filter(|s| !s.is_empty())
		.filter_map(|pair| {
			let (key, value) = pair.split_once('=')?;
			Some((key.to_string(), value.to_string()))
		})
		.collect();

	Ok((path.to_string(), params))
}

/// Synchronous wrapper for fetching a secret (blocks on async runtime)
pub fn fetch_secret_blocking(uri: &str) -> Result<String, SecretProviderError> {
	let provider = parse_secret_uri(uri)?;

	// Use tokio runtime if available, otherwise create a new one
	if let Ok(handle) = tokio::runtime::Handle::try_current() {
		// We're in an async context, use block_in_place
		tokio::task::block_in_place(|| handle.block_on(provider.get_secret()))
	} else {
		// Create a new runtime
		let rt =
			tokio::runtime::Builder::new_current_thread()
				.enable_all()
				.build()
				.map_err(|e| {
					SecretProviderError::ConfigError(format!("Failed to create runtime: {e}"))
				})?;
		rt.block_on(provider.get_secret())
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_parse_plain_file_path() {
		// Just verify it parses without error
		let _provider = parse_secret_uri("/path/to/secret").unwrap();
	}

	#[test]
	fn test_parse_file_uri() {
		let _provider = parse_secret_uri("file:///path/to/secret").unwrap();
	}

	#[test]
	fn test_parse_aws_uri() {
		let _provider = parse_secret_uri("aws://my-secret").unwrap();
	}

	#[test]
	fn test_parse_aws_uri_with_params() {
		let _provider =
			parse_secret_uri("aws://my-secret?region=us-west-2&version_id=abc123").unwrap();
	}

	#[test]
	fn test_parse_aws_uri_with_field() {
		let _provider =
			parse_secret_uri("aws://my-secret?region=us-east-1&field=aura_key").unwrap();
	}

	#[test]
	fn test_parse_gcp_uri() {
		let _provider =
			parse_secret_uri("gcp://projects/my-project/secrets/my-secret/versions/latest")
				.unwrap();
	}

	#[test]
	fn test_parse_gcp_uri_with_field() {
		let _provider =
			parse_secret_uri("gcp://projects/proj/secrets/sec/versions/1?field=key").unwrap();
	}

	#[test]
	fn test_parse_vault_uri() {
		let _provider = parse_secret_uri("vault://secret/data/my-app#seed").unwrap();
	}

	#[test]
	fn test_parse_vault_uri_with_query_field() {
		let _provider = parse_secret_uri("vault://secret/path?field=mykey").unwrap();
	}

	#[test]
	fn test_unsupported_scheme() {
		let result = parse_secret_uri("unknown://something");
		assert!(matches!(result, Err(SecretProviderError::UnsupportedScheme(_))));
	}
}
