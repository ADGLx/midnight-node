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

//! File-based secret provider (the original implementation)

use super::{SecretProvider, SecretProviderError};

/// Provider for reading secrets from local files
pub struct FileSecretProvider {
	path: String,
}

impl FileSecretProvider {
	pub fn new(path: String) -> Self {
		Self { path }
	}
}

#[async_trait::async_trait]
impl SecretProvider for FileSecretProvider {
	async fn get_secret(&self) -> Result<String, SecretProviderError> {
		let content = tokio::fs::read_to_string(&self.path).await?;
		Ok(content.trim().to_string())
	}
}

