// This file is part of midnight-node.
// Copyright (C) 2025 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
// http://www.apache.org/licenses/LICENSE-2.0
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

import { execFileSync } from "child_process";

interface SdmResource {
  name: string;
  type: string;
  address: string;
  status: string;
  localAddress?: string;
}

interface HostTarget {
  hostname: string;
  envKeys: string[];
  connectionString: string;
}

export async function setupSdmProxies(
  env: Record<string, string>,
  _namespace: string,
): Promise<Record<string, string>> {
  const targets = extractHostTargets(env);
  if (targets.length === 0) {
    console.log("No DB connection strings found for StrongDM proxying");
    return {};
  }

  console.log(
    `Setting up StrongDM proxies for ${targets.length} unique Aurora host(s)`,
  );

  const resources = querySdmResources();
  const overrides: Record<string, string> = {};

  for (const target of targets) {
    const resource = findSdmResourceByHostname(resources, target.hostname);
    if (!resource) {
      console.warn(
        `No StrongDM resource found for hostname ${target.hostname}; skipping`,
      );
      continue;
    }

    if (resource.status !== "connected") {
      ensureSdmConnected(resource.name);
    }

    const localAddress = getSdmLocalAddress(resource.name);
    if (!localAddress) {
      console.warn(
        `Could not determine StrongDM local address for ${resource.name}; skipping`,
      );
      continue;
    }

    const sdmPort = parsePortFromAddress(localAddress);
    if (!sdmPort) {
      console.warn(
        `Could not parse port from StrongDM local address '${localAddress}'; skipping`,
      );
      continue;
    }

    console.log(
      `StrongDM: ${resource.name} → localhost:${sdmPort} (for ${target.hostname})`,
    );

    for (const envKey of target.envKeys) {
      overrides[envKey] = rewriteConnectionStringForSdm(
        env[envKey],
        sdmPort,
      );
    }
  }

  return overrides;
}

function extractHostTargets(env: Record<string, string>): HostTarget[] {
  const byHostname: Record<string, HostTarget> = {};
  const regex =
    /^DB_SYNC_POSTGRES_CONNECTION_STRING_(?:BOOT_|NODE_)?MIDNIGHT_NODE_(?:BOOT_)?(\d+)_0$/;

  for (const [envKey, connString] of Object.entries(env)) {
    if (!envKey.match(regex) || !connString) {
      continue;
    }

    let hostname = "";
    try {
      const url = new URL(connString.replace(/^psql:/, "postgres:"));
      hostname = url.hostname;
    } catch {
      continue;
    }

    if (!hostname) {
      continue;
    }

    if (!byHostname[hostname]) {
      byHostname[hostname] = { hostname, envKeys: [], connectionString: connString };
    }
    byHostname[hostname].envKeys.push(envKey);
  }

  return Object.values(byHostname);
}

function querySdmResources(): SdmResource[] {
  try {
    const output = execFileSync(
      "sdm",
      ["status", "--json", "--filter", "type:aurorapostgresiam"],
      { encoding: "utf-8", stdio: ["pipe", "pipe", "pipe"] },
    );
    return JSON.parse(output) as SdmResource[];
  } catch (error) {
    console.warn(
      `Failed to query StrongDM resources: ${(error as Error).message}`,
    );
    return [];
  }
}

function findSdmResourceByHostname(
  resources: SdmResource[],
  hostname: string,
): SdmResource | undefined {
  return resources.find((r) => r.address?.includes(hostname));
}

function ensureSdmConnected(resourceName: string): void {
  console.log(`Connecting to StrongDM resource: ${resourceName}`);
  try {
    execFileSync("sdm", ["connect", resourceName], {
      stdio: ["pipe", "inherit", "inherit"],
      timeout: 30_000,
    });
  } catch (error) {
    console.warn(
      `Failed to connect StrongDM resource '${resourceName}': ${(error as Error).message}`,
    );
  }
}

function getSdmLocalAddress(resourceName: string): string | undefined {
  try {
    const output = execFileSync(
      "sdm",
      ["status", "--json", "--filter", `name:${resourceName}`],
      { encoding: "utf-8", stdio: ["pipe", "pipe", "pipe"] },
    );
    const resources = JSON.parse(output) as SdmResource[];
    const resource = resources.find((r) => r.name === resourceName);
    return resource?.localAddress;
  } catch {
    return undefined;
  }
}

function parsePortFromAddress(address: string): number | undefined {
  const colonIdx = address.lastIndexOf(":");
  if (colonIdx === -1) {
    return undefined;
  }
  const port = parseInt(address.slice(colonIdx + 1), 10);
  return Number.isNaN(port) ? undefined : port;
}

function rewriteConnectionStringForSdm(
  connString: string,
  sdmPort: number,
): string {
  try {
    const url = new URL(connString.replace(/^psql:/, "postgres:"));
    url.hostname = "host.docker.internal";
    url.port = `${sdmPort}`;
    return url.toString();
  } catch (error) {
    console.warn(
      `Failed to rewrite connection string: ${(error as Error).message}`,
    );
    return connString;
  }
}
