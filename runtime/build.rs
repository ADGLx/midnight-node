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

fn main() {
	#[cfg(feature = "std")]
	{
		substrate_wasm_builder::WasmBuilder::new()
			.with_current_project()
			.export_heap_base()
			.import_memory()
			.build();
	}

	// When std is not enabled (e.g. Bazel builds that skip wasm-builder to avoid
	// the wasm-opt-sys/scratch/cxx sandbox symlink issue), check if a pre-built
	// WASM blob is available via WASM_BINARY_PATH, otherwise generate a stub.
	#[cfg(not(feature = "std"))]
	{
		let out = std::env::var("OUT_DIR").unwrap();
		let out_path = std::path::Path::new(&out);

		if let Ok(wasm_path) = std::env::var("WASM_BINARY_PATH") {
			println!("cargo:rerun-if-env-changed=WASM_BINARY_PATH");
			let wasm = std::fs::read(&wasm_path)
				.unwrap_or_else(|e| panic!("Failed to read WASM at {wasm_path}: {e}"));
			let dest = out_path.join("wasm_binary.wasm");
			std::fs::write(&dest, &wasm).unwrap();
			std::fs::write(
				out_path.join("wasm_binary.rs"),
				"pub const WASM_BINARY: Option<&[u8]> = \
				 Some(include_bytes!(\"wasm_binary.wasm\"));\n\
				 pub const WASM_BINARY_BLOATY: Option<&[u8]> = \
				 Some(include_bytes!(\"wasm_binary.wasm\"));\n",
			)
			.unwrap();
		} else {
			std::fs::write(
				out_path.join("wasm_binary.rs"),
				"pub const WASM_BINARY: Option<&[u8]> = None;\n\
				 pub const WASM_BINARY_BLOATY: Option<&[u8]> = None;\n",
			)
			.unwrap();
		}
	}
}
