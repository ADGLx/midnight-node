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

//! Contract modules that require ledger 7.x APIs.
//! These are only available in the `latest` version module.

#[cfg(feature = "can-panic")]
mod maintenance;
#[cfg(feature = "can-panic")]
mod merkle_tree;

#[cfg(feature = "can-panic")]
pub use maintenance::*;
#[cfg(feature = "can-panic")]
pub use merkle_tree::*;

