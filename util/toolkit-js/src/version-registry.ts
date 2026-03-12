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

const VERSION_MAP: Record<string, string> = {
  '0.29.0': '@midnight-ntwrk/compact-js-command',
  '0.30.0-rc.0': '@midnight-ntwrk/compact-js-command-v2-5-0',
};

const DEFAULT_VERSION = '0.30.0-rc.0';

export function getCommandPackage(compactVersion: string): string {
  const pkg = VERSION_MAP[compactVersion];
  if (!pkg) {
    const supported = getSupportedVersions().join(', ');
    throw new Error(
      `Unsupported compact version: ${compactVersion}. Supported versions: ${supported}`
    );
  }
  return pkg;
}

export function getSupportedVersions(): string[] {
  return Object.keys(VERSION_MAP);
}

export function getDefaultVersion(): string {
  return DEFAULT_VERSION;
}
