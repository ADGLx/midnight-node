// This file is part of midnight-node.
// Copyright (C) 2025-2026 Midnight Foundation
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

import { execFileSync, execSync } from "child_process";
import { readFileSync } from "fs";
import { loadNetworkConfig } from "./networkConfig";

/** Pod port map e.g. { "psql-dbsync-cardano-0-db-01": 54321 } */
type PortMapping = Record<string, number>;

interface PostgresSecret {
  host: string;
  password: string;
  port: string;
  user: string;
  db: string;
  connectionString?: string;
}

// Roles according to the running networks
type PodNodeRole = "authority" | "boot";

interface NodeSecrets {
  seed?: string;
  auraSeed?: string;
  grandpaSeed?: string;
  crossChainSeed?: string;
  postgres?: PostgresSecret;
  role: PodNodeRole;
  envPrefix?: string;
}

type SecretsByNode = Record<string, NodeSecrets>;

const AUTHORITY_ENV_FIELDS = [
  "SEED_PHRASE",
  "AURA_SEED_FILE",
  "GRANDPA_SEED_FILE",
  "CROSS_CHAIN_SEED_FILE",
  "POSTGRES_HOST",
  "POSTGRES_PASSWORD",
  "POSTGRES_PORT",
  "POSTGRES_USER",
  "POSTGRES_DB",
] as const;

const BOOT_ENV_FIELDS = [
  "POSTGRES_HOST",
  "POSTGRES_PASSWORD",
  "POSTGRES_PORT",
  "POSTGRES_USER",
  "POSTGRES_DB",
] as const;

const SEED_ENV_KEYS = [
  ["seed", "SEED"],
  ["auraSeed", "AURA_SEED"],
  ["grandpaSeed", "GRANDPA_SEED"],
  ["crossChainSeed", "CROSS_CHAIN_SEED"],
] as const;

const AWS_QANET_EXPECTED_NODE_COUNT = 12;

interface AwsCliContext {
  profile: string;
  region: string;
}

interface QanetSeedSet {
  seed?: string;
  auraSeed?: string;
  grandpaSeed?: string;
  crossChainSeed?: string;
}

interface AwsSecretsListResponse {
  SecretList?: Array<{
    Name?: string;
  }>;
}

interface AwsSecretValueResponse {
  SecretString?: string;
}

interface AwsDescribeDbClustersResponse {
  DBClusters?: Array<{
    DBClusterIdentifier?: string;
    Endpoint?: string;
    Port?: number;
    MasterUsername?: string;
    DatabaseName?: string;
    Status?: string;
  }>;
}

interface AwsDescribeInstancesResponse {
  Reservations?: Array<{
    Instances?: Array<{
      Tags?: Array<{
        Key?: string;
        Value?: string;
      }>;
    }>;
  }>;
}

// TODO: Change this to use AWS SSM
export function getSecrets(namespace: string): Record<string, string> {
  const networkConfig = loadNetworkConfig(namespace);

  if (networkConfig.secrets.mode === "k8s-secret") {
    return getK8sSecretSecrets(namespace);
  }

  if (networkConfig.secrets.mode === "aws-qanet") {
    return getAwsQanetSecrets();
  }

  if (networkConfig.secrets.mode === "preview-style") {
    return getPreviewSecrets(namespace);
  }

  const portMapping = loadPortMapping();

  const secrets: SecretsByNode = {};
  collectAuthorityPods(namespace, portMapping, secrets);
  collectBootPods(namespace, portMapping, secrets, networkConfig.boot.podNames);

  const envObject = convertSecretsToEnvObject(secrets);
  return envObject;
}

const K8S_SECRET_EXPECTED_NODE_COUNT = 12;

interface K8sSecretResponse {
  data?: Record<string, string>;
}

function getK8sSecretSecrets(namespace: string): Record<string, string> {
  const secretName =
    firstNonEmpty([process.env.MN_QANET_K8S_SECRET_NAME]) ??
    `${namespace}-node-seed-phrases`;

  console.log(
    `loading seed phrases from k8s secret '${secretName}' in namespace '${namespace}'`,
  );

  const seedPayload = loadK8sSecretData(namespace, secretName);
  const seedsByNode = parseQanetSeedPayload(seedPayload);
  const nodeIndexes = Array.from(seedsByNode.keys()).sort((a, b) => a - b);

  if (nodeIndexes.length === 0) {
    throw new Error(
      `k8s secret '${secretName}' did not contain any node seed entries`,
    );
  }

  if (nodeIndexes.length < K8S_SECRET_EXPECTED_NODE_COUNT) {
    console.warn(
      `k8s secret has ${nodeIndexes.length} node(s); compose expects ${K8S_SECRET_EXPECTED_NODE_COUNT}. proceeding with available nodes.`,
    );
  }

  const connectionString = buildK8sSecretConnectionString();

  const env: Record<string, string> = {};

  for (const nodeIndex of nodeIndexes) {
    const seedSet = seedsByNode.get(nodeIndex);
    if (!seedSet) {
      continue;
    }

    const prefix = `MIDNIGHT_NODE_${padNodeIndex(nodeIndex)}_0`;
    if (seedSet.seed) {
      env[`${prefix}_SEED`] = seedSet.seed;
    }
    if (seedSet.auraSeed) {
      env[`${prefix}_AURA_SEED`] = seedSet.auraSeed;
    }
    if (seedSet.grandpaSeed) {
      env[`${prefix}_GRANDPA_SEED`] = seedSet.grandpaSeed;
    }
    if (seedSet.crossChainSeed) {
      env[`${prefix}_CROSS_CHAIN_SEED`] = seedSet.crossChainSeed;
    }
  }

  const maxNodeIndex = Math.max(
    K8S_SECRET_EXPECTED_NODE_COUNT,
    nodeIndexes[nodeIndexes.length - 1] ?? K8S_SECRET_EXPECTED_NODE_COUNT,
  );

  for (let index = 1; index <= maxNodeIndex; index += 1) {
    env[
      `DB_SYNC_POSTGRES_CONNECTION_STRING_NODE_MIDNIGHT_NODE_${padNodeIndex(index)}_0`
    ] = connectionString;
  }

  return env;
}

function loadK8sSecretData(
  namespace: string,
  secretName: string,
): Record<string, string> {
  const cmd = `kubectl get secret ${secretName} -n ${namespace} -o json`;

  let raw: string;
  try {
    raw = execSync(cmd, { encoding: "utf-8" }).trim();
  } catch (error) {
    throw new Error(
      `failed to read k8s secret '${secretName}' in namespace '${namespace}': ${(error as Error).message}`,
    );
  }

  const parsed = JSON.parse(raw) as K8sSecretResponse;
  const data = parsed.data ?? {};

  const decoded: Record<string, string> = {};
  for (const [key, value] of Object.entries(data)) {
    const decodedValue = Buffer.from(value, "base64").toString("utf-8").trim();
    if (!decodedValue) {
      continue;
    }

    // Support both individual keys (node-1, node-1-aura, etc.)
    // and a single JSON blob key (e.g. "seeds" containing the full payload)
    try {
      const jsonBlob = JSON.parse(decodedValue) as Record<string, unknown>;
      if (typeof jsonBlob === "object" && jsonBlob !== null && !Array.isArray(jsonBlob)) {
        for (const [blobKey, blobValue] of Object.entries(jsonBlob)) {
          if (typeof blobValue === "string") {
            decoded[blobKey] = blobValue.trim();
          }
        }
        continue;
      }
    } catch {
      // Not JSON, treat as a plain string value
    }

    decoded[key] = decodedValue;
  }

  return decoded;
}

function buildK8sSecretConnectionString(): string {
  const overrideConnectionString = process.env.MN_QANET_DB_CONNECTION_STRING;
  if (overrideConnectionString?.trim()) {
    console.log("using override DB connection string for qanet");
    return overrideConnectionString.trim();
  }

  const host =
    firstNonEmpty([process.env.MN_QANET_DB_HOST]) ??
    discoverPostgresHostFromSdm();

  if (!host) {
    throw new Error(
      "unable to discover DB host. ensure an SDM postgres resource is available, or set MN_QANET_DB_CONNECTION_STRING.",
    );
  }

  const database =
    firstNonEmpty([process.env.MN_QANET_DB_NAME]) ?? "cexplorer";

  const portRaw = firstNonEmpty([process.env.MN_QANET_DB_PORT]) ?? "5432";
  const port = Number.parseInt(portRaw, 10);
  if (!Number.isFinite(port) || port <= 0) {
    throw new Error(`invalid qanet DB port '${portRaw}'`);
  }

  const user = firstNonEmpty([process.env.MN_QANET_DB_USER]) ?? "cardano";
  const encodedUser = encodeURIComponent(user);
  const encodedDb = encodeURIComponent(database);

  // No password — SDM handles authentication
  return `psql://${encodedUser}@${host}:${port}/${encodedDb}`;
}

interface SdmStatusEntry {
  name: string;
  type: string;
  address: string;
  hostname: string;
  tags: string;
  connected: boolean;
}

function discoverPostgresHostFromSdm(): string | undefined {
  console.log("discovering postgres host from StrongDM...");

  let entries: SdmStatusEntry[];
  try {
    const output = execFileSync(
      "sdm",
      ["status", "--json"],
      { encoding: "utf-8", stdio: ["pipe", "pipe", "pipe"] },
    );
    entries = JSON.parse(output) as SdmStatusEntry[];
  } catch (error) {
    console.warn(
      `failed to query StrongDM resources: ${(error as Error).message}`,
    );
    return undefined;
  }

  // Find a resource tagged with chain_name=qanet and an aurora/postgres type
  const qanetResource = entries.find(
    (r) =>
      /chain_name=qanet/i.test(r.tags ?? "") &&
      /aurora|postgres/i.test(r.type ?? ""),
  );

  if (!qanetResource) {
    console.warn(
      "no StrongDM resource found with chain_name=qanet tag",
    );
    return undefined;
  }

  // The hostname field contains the actual remote DB host
  const hostname = qanetResource.hostname;
  if (!hostname) {
    console.warn(
      `StrongDM resource '${qanetResource.name}' has no hostname`,
    );
    return undefined;
  }

  console.log(
    `discovered postgres host '${hostname}' from StrongDM resource '${qanetResource.name}'`,
  );
  return hostname;
}

function getAwsQanetSecrets(): Record<string, string> {
  const aws = resolveQanetAwsContext();
  console.log(
    `loading qanet secrets from AWS (profile='${aws.profile}', region='${aws.region}')`,
  );

  const seedSecretCandidates = compactStrings([
    process.env.MN_AWS_QANET_SEED_SECRET,
    process.env.MN_AWS_QANET_SEED_SECRET_FALLBACK,
  ]);

  if (seedSecretCandidates.length === 0) {
    throw new Error(
      "no AWS seed secret configured. set MN_AWS_QANET_SEED_SECRET or MN_AWS_QANET_SEED_SECRET_FALLBACK.",
    );
  }

  const { secretId: seedSecretId, value: seedPayload } =
    loadFirstExistingJsonSecret(seedSecretCandidates, aws);
  console.log(`using AWS seed secret '${seedSecretId}'`);

  const seedsByNode = parseQanetSeedPayload(seedPayload);
  const nodeIndexes = Array.from(seedsByNode.keys()).sort((a, b) => a - b);
  if (nodeIndexes.length === 0) {
    throw new Error(
      `seed secret '${seedSecretId}' did not contain any node seed entries`,
    );
  }

  if (nodeIndexes.length < AWS_QANET_EXPECTED_NODE_COUNT) {
    console.warn(
      `qanet AWS seed set has ${nodeIndexes.length} node(s); compose expects ${AWS_QANET_EXPECTED_NODE_COUNT}. proceeding with available nodes.`,
    );
  }

  const connectionString = buildQanetConnectionString(aws);

  const env: Record<string, string> = {};

  for (const nodeIndex of nodeIndexes) {
    const seedSet = seedsByNode.get(nodeIndex);
    if (!seedSet) {
      continue;
    }

    const prefix = `MIDNIGHT_NODE_${padNodeIndex(nodeIndex)}_0`;
    if (seedSet.seed) {
      env[`${prefix}_SEED`] = seedSet.seed;
    }
    if (seedSet.auraSeed) {
      env[`${prefix}_AURA_SEED`] = seedSet.auraSeed;
    }
    if (seedSet.grandpaSeed) {
      env[`${prefix}_GRANDPA_SEED`] = seedSet.grandpaSeed;
    }
    if (seedSet.crossChainSeed) {
      env[`${prefix}_CROSS_CHAIN_SEED`] = seedSet.crossChainSeed;
    }
  }

  const maxNodeIndex = Math.max(
    AWS_QANET_EXPECTED_NODE_COUNT,
    nodeIndexes[nodeIndexes.length - 1] ?? AWS_QANET_EXPECTED_NODE_COUNT,
  );

  for (let index = 1; index <= maxNodeIndex; index += 1) {
    env[
      `DB_SYNC_POSTGRES_CONNECTION_STRING_NODE_MIDNIGHT_NODE_${padNodeIndex(index)}_0`
    ] = connectionString;
  }

  return env;
}

function resolveQanetAwsContext(): AwsCliContext {
  const profile = firstNonEmpty([
    process.env.MN_AWS_QANET_PROFILE,
    process.env.MN_AWS_PROFILE_QANET,
    process.env.AWS_PROFILE,
  ]);

  if (!profile) {
    throw new Error(
      "no AWS profile configured. set MN_AWS_QANET_PROFILE, MN_AWS_PROFILE_QANET, or AWS_PROFILE.",
    );
  }

  const region = firstNonEmpty([
    process.env.MN_AWS_QANET_REGION,
    process.env.MN_AWS_REGION_QANET,
    process.env.MN_AWS_REGION,
    process.env.AWS_REGION,
    process.env.AWS_DEFAULT_REGION,
  ]);

  if (!region) {
    throw new Error(
      "no AWS region configured. set MN_AWS_QANET_REGION, AWS_REGION, or AWS_DEFAULT_REGION.",
    );
  }

  return { profile, region };
}

function buildQanetConnectionString(aws: AwsCliContext): string {
  const overrideConnectionString = process.env.MN_AWS_QANET_DB_CONNECTION_STRING;
  if (overrideConnectionString?.trim()) {
    console.log("using override DB connection string for qanet");
    return overrideConnectionString.trim();
  }

  const groupHint =
    firstNonEmpty([process.env.MN_AWS_QANET_GROUP_PET]) ??
    resolveQanetGroupPetFromEc2(aws);
  if (groupHint) {
    console.log(`resolved qanet resource group '${groupHint}'`);
  }

  const auroraSecretId =
    firstNonEmpty([process.env.MN_AWS_QANET_AURORA_SECRET]) ??
    resolveQanetAuroraSecretName(aws, groupHint);

  const auroraSecret = loadJsonSecret(auroraSecretId, aws);
  const cluster = resolveQanetCluster(aws, groupHint);

  const host =
    firstNonEmpty([process.env.MN_AWS_QANET_DB_HOST, cluster.Endpoint]) ?? "";
  if (!host) {
    throw new Error("unable to resolve AWS qanet DB host");
  }

  const username =
    firstNonEmpty([
      process.env.MN_AWS_QANET_DB_USER,
      auroraSecret.username,
      cluster.MasterUsername,
    ]) ?? "cardano";

  const database =
    firstNonEmpty([
      process.env.MN_AWS_QANET_DB_NAME,
      cluster.DatabaseName,
      "cexplorer",
    ]) ?? "cexplorer";

  const password = firstNonEmpty([
    process.env.MN_AWS_QANET_DB_PASSWORD,
    auroraSecret.password,
  ]);
  if (!password) {
    throw new Error(
      `unable to resolve DB password (checked secret '${auroraSecretId}' and MN_AWS_QANET_DB_PASSWORD)`,
    );
  }

  const portRaw =
    firstNonEmpty([
      process.env.MN_AWS_QANET_DB_PORT,
      cluster.Port ? String(cluster.Port) : undefined,
    ]) ?? "5432";
  const port = Number.parseInt(portRaw, 10);
  if (!Number.isFinite(port) || port <= 0) {
    throw new Error(`invalid AWS qanet DB port '${portRaw}'`);
  }

  const encodedUser = encodeURIComponent(username);
  const encodedPassword = encodeURIComponent(password);
  const encodedDb = encodeURIComponent(database);

  return `psql://${encodedUser}:${encodedPassword}@${host}:${port}/${encodedDb}?sslmode=require`;
}

function resolveQanetAuroraSecretName(
  aws: AwsCliContext,
  groupHint?: string,
): string {
  const response = awsCliJson<AwsSecretsListResponse>(
    ["secretsmanager", "list-secrets"],
    aws,
  );

  const names = (response.SecretList ?? [])
    .map((secret) => secret.Name?.trim())
    .filter((name): name is string => Boolean(name))
    .filter((name) => name.includes("qanet") && name.includes("aurora-master"));

  if (names.length === 0) {
    throw new Error(
      "could not find a qanet aurora master secret in AWS Secrets Manager",
    );
  }

  if (groupHint) {
    const exactPath = `qanet/midnight/${groupHint}/aurora-master`;
    const exactMatch = names.find((name) => name === exactPath);
    if (exactMatch) {
      return exactMatch;
    }

    const groupMatches = names.filter((name) =>
      name.includes(`-${groupHint}-aurora-master`),
    );
    if (groupMatches.length === 1) {
      return groupMatches[0];
    }
    if (groupMatches.length > 1) {
      throw new Error(
        `multiple aurora secrets matched group '${groupHint}': ${groupMatches.join(", ")}`,
      );
    }
  }

  const sorted = names.sort((a, b) => a.localeCompare(b));
  const preferred =
    sorted.find((name) => name.startsWith("qanet/midnight/")) ?? sorted[0];

  return preferred;
}

function resolveQanetCluster(
  aws: AwsCliContext,
  groupHint?: string,
): NonNullable<AwsDescribeDbClustersResponse["DBClusters"]>[number] {
  const response = awsCliJson<AwsDescribeDbClustersResponse>(
    ["rds", "describe-db-clusters"],
    aws,
  );

  const clusters = (response.DBClusters ?? []).filter((cluster) =>
    cluster.DBClusterIdentifier?.includes("midnight-qanet"),
  );

  if (clusters.length === 0) {
    throw new Error("could not find an RDS cluster matching 'midnight-qanet'");
  }

  if (groupHint) {
    const exactId = `midnight-qanet-${groupHint}`;
    const exact = clusters.find(
      (cluster) => cluster.DBClusterIdentifier === exactId,
    );
    if (exact) {
      return exact;
    }
  }

  const available =
    clusters.find(
      (cluster) => cluster.Status === "available" && Boolean(cluster.Endpoint),
    ) ??
    clusters.find((cluster) => Boolean(cluster.Endpoint)) ??
    clusters[0];

  return available;
}

function resolveQanetGroupPetFromEc2(
  aws: AwsCliContext,
): string | undefined {
  const response = awsCliJson<AwsDescribeInstancesResponse>(
    [
      "ec2",
      "describe-instances",
      "--filters",
      "Name=instance-state-name,Values=running",
      "Name=tag:chain_name,Values=qanet",
    ],
    aws,
  );

  const instanceNameTags = (response.Reservations ?? [])
    .flatMap((reservation) => reservation.Instances ?? [])
    .map((instance) => tagValue(instance.Tags, "Name"))
    .filter((name): name is string => Boolean(name));

  const validatorGroups = new Set<string>();
  const fallbackGroups = new Set<string>();

  for (const name of instanceNameTags) {
    const validatorMatch = name.match(/^qanet-vali-([^-]+)-/i);
    if (validatorMatch) {
      validatorGroups.add(validatorMatch[1]);
      continue;
    }

    const cardanoMatch = name.match(/^cardano-qanet-([^-]+)$/i);
    if (cardanoMatch) {
      fallbackGroups.add(cardanoMatch[1]);
    }
  }

  if (validatorGroups.size === 1) {
    return Array.from(validatorGroups)[0];
  }
  if (validatorGroups.size > 1) {
    throw new Error(
      `multiple running validator groups detected for qanet: ${Array.from(validatorGroups).join(", ")}. set MN_AWS_QANET_GROUP_PET.`,
    );
  }

  if (fallbackGroups.size === 1) {
    return Array.from(fallbackGroups)[0];
  }
  if (fallbackGroups.size > 1) {
    throw new Error(
      `multiple running fallback groups detected for qanet: ${Array.from(fallbackGroups).join(", ")}. set MN_AWS_QANET_GROUP_PET.`,
    );
  }

  return undefined;
}

function loadFirstExistingJsonSecret(
  secretIds: string[],
  aws: AwsCliContext,
): { secretId: string; value: Record<string, string> } {
  const errors: string[] = [];

  for (const secretId of secretIds) {
    try {
      return {
        secretId,
        value: loadJsonSecret(secretId, aws),
      };
    } catch (error) {
      errors.push(`${secretId}: ${(error as Error).message}`);
    }
  }

  throw new Error(
    `failed to load any candidate secret. attempts:\n${errors.join("\n")}`,
  );
}

function loadJsonSecret(
  secretId: string,
  aws: AwsCliContext,
): Record<string, string> {
  const response = awsCliJson<AwsSecretValueResponse>(
    ["secretsmanager", "get-secret-value", "--secret-id", secretId],
    aws,
  );

  const raw = response.SecretString?.trim();
  if (!raw || raw === "None") {
    throw new Error(`secret '${secretId}' is empty or missing SecretString`);
  }

  let parsed: unknown;
  try {
    parsed = JSON.parse(raw);
  } catch (error) {
    throw new Error(
      `secret '${secretId}' does not contain JSON payload: ${(error as Error).message}`,
    );
  }

  if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) {
    throw new Error(`secret '${secretId}' did not parse to an object`);
  }

  const out: Record<string, string> = {};
  for (const [key, value] of Object.entries(parsed as Record<string, unknown>)) {
    if (typeof value === "string") {
      out[key] = value.trim();
      continue;
    }
    if (value === null || value === undefined) {
      continue;
    }
    out[key] = String(value);
  }

  return out;
}

function parseQanetSeedPayload(seedPayload: Record<string, string>) {
  const seedsByNode = new Map<number, QanetSeedSet>();

  const ensureSeedSet = (index: number) => {
    let current = seedsByNode.get(index);
    if (!current) {
      current = {};
      seedsByNode.set(index, current);
    }
    return current;
  };

  for (const [key, value] of Object.entries(seedPayload)) {
    const trimmed = value?.trim();
    if (!trimmed) {
      continue;
    }

    const categoryMatch = key.match(/^node-(\d+)-(aura|grandpa|cross-chain)$/i);
    if (categoryMatch) {
      const index = Number.parseInt(categoryMatch[1], 10);
      if (!Number.isFinite(index)) {
        continue;
      }

      const entry = ensureSeedSet(index);
      const category = categoryMatch[2].toLowerCase();
      if (category === "aura") {
        entry.auraSeed = trimmed;
      } else if (category === "grandpa") {
        entry.grandpaSeed = trimmed;
      } else if (category === "cross-chain") {
        entry.crossChainSeed = trimmed;
      }
      continue;
    }

    const legacyMatch = key.match(/^node-(\d+)$/i);
    if (!legacyMatch) {
      continue;
    }

    const index = Number.parseInt(legacyMatch[1], 10);
    if (!Number.isFinite(index)) {
      continue;
    }

    const entry = ensureSeedSet(index);
    entry.seed = trimmed;
  }

  return seedsByNode;
}

function awsCliJson<T>(args: string[], aws: AwsCliContext): T {
  const raw = execFileSync(
    "aws",
    [...args, "--region", aws.region, "--profile", aws.profile, "--output", "json"],
    {
      encoding: "utf-8",
    },
  ).trim();

  if (!raw) {
    throw new Error(`aws ${args.join(" ")} returned empty output`);
  }

  return JSON.parse(raw) as T;
}

function padNodeIndex(index: number): string {
  return index.toString().padStart(2, "0");
}

function compactStrings(values: Array<string | undefined>): string[] {
  return values
    .map((value) => value?.trim())
    .filter((value): value is string => Boolean(value));
}

function firstNonEmpty(values: Array<string | undefined>): string | undefined {
  for (const value of values) {
    if (value?.trim()) {
      return value.trim();
    }
  }
  return undefined;
}

function tagValue(
  tags: Array<{ Key?: string; Value?: string }> | undefined,
  key: string,
): string | undefined {
  return tags?.find((tag) => tag.Key === key)?.Value;
}

function loadPortMapping(): PortMapping {
  console.log("loading port mapping from port-mapping.json");
  try {
    const portMappingRaw = readFileSync("port-mapping.json", "utf-8");
    const portMapping = JSON.parse(portMappingRaw) as PortMapping;
    console.log(
      `loaded ${Object.keys(portMapping).length} port mapping entries`,
    );
    return portMapping;
  } catch (error) {
    throw new Error(
      `failed to read port-mapping.json: ${(error as Error).message}`,
    );
  }
}

function collectAuthorityPods(
  namespace: string,
  portMapping: PortMapping,
  secrets: SecretsByNode,
) {
  const pods = listPods(namespace, "midnight.tech/node-type=authority");
  console.log(`processing ${pods.length} authority pod(s)`);

  for (const pod of pods) {
    const envValues = readPodEnv(namespace, pod, AUTHORITY_ENV_FIELDS);
    const nodeKey = formatNodeKey(pod);

    const seed = envValues.SEED_PHRASE?.trim() || undefined;

    const auraSeed = readSeedFile(
      namespace,
      pod,
      envValues.AURA_SEED_FILE,
      "aura",
    );
    const grandpaSeed = readSeedFile(
      namespace,
      pod,
      envValues.GRANDPA_SEED_FILE,
      "grandpa",
    );
    const crossChainSeed = readSeedFile(
      namespace,
      pod,
      envValues.CROSS_CHAIN_SEED_FILE,
      "cross-chain",
    );

    secrets[nodeKey] = {
      seed,
      auraSeed,
      grandpaSeed,
      crossChainSeed,
      postgres: buildPostgresSecret(envValues, portMapping),
      role: "authority",
    };
  }
}

function collectBootPods(
  namespace: string,
  portMapping: PortMapping,
  secrets: SecretsByNode,
  explicitPods: string[] = [],
) {
  const pods =
    explicitPods.length > 0
      ? explicitPods
      : listPods(namespace, "midnight.tech/node-type=boot");
  console.log(`processing ${pods.length} boot pod(s)`);

  for (const pod of pods) {
    const envValues = readPodEnv(namespace, pod, BOOT_ENV_FIELDS);
    const nodeKey = formatNodeKey(pod);

    secrets[nodeKey] = {
      postgres: buildPostgresSecret(envValues, portMapping),
      role: "boot",
    };
  }
}

function buildPostgresSecret(
  envValues: Record<string, string>,
  portMapping: PortMapping,
): PostgresSecret | undefined {
  const host = envValues.POSTGRES_HOST?.trim() ?? "";
  const password = envValues.POSTGRES_PASSWORD?.trim() ?? "";
  const port = envValues.POSTGRES_PORT?.trim() ?? "";
  const user = envValues.POSTGRES_USER?.trim() ?? "";
  const db = envValues.POSTGRES_DB?.trim() ?? "";

  if (!(host || password || port || user || db)) {
    return undefined;
  }

  const secret: PostgresSecret = {
    host,
    password,
    port,
    user,
    db,
  };

  const mappedPort = host ? getPortFromMapping(host, portMapping) : undefined;
  if (mappedPort) {
    secret.connectionString = `psql://${user}:${password}@host.docker.internal:${mappedPort}/${db}?sslmode=disable`;
  }
  return secret;
}

function readPodEnv(
  namespace: string,
  pod: string,
  fields: readonly string[],
): Record<string, string> {
  if (fields.length === 0) {
    return {};
  }

  const echoExpr = fields.map((field) => `$${field}`).join("|");
  const cmd = `kubectl exec -n ${namespace} ${pod} -- sh -c 'echo "${echoExpr}"'`;

  try {
    const raw = execSync(cmd, { encoding: "utf-8" }).trim();
    const pieces = raw ? raw.split("|") : [];

    return Object.fromEntries(
      fields.map((field, index) => [field, (pieces[index] ?? "").trim()]),
    );
  } catch (error) {
    console.warn(
      `pod '${pod}' failed to read env fields [${fields.join(", ")}]: ${(error as Error).message}`,
    );
    return Object.fromEntries(fields.map((field) => [field, ""]));
  }
}

function readSeedFile(
  namespace: string,
  pod: string,
  filePath: string | undefined,
  label: string,
): string | undefined {
  const trimmed = filePath?.trim();
  if (!trimmed) {
    return undefined;
  }

  try {
    const cmd = `kubectl exec -n ${namespace} ${pod} -- sh -c 'cat "${trimmed}"'`;
    const seed = execSync(cmd, { encoding: "utf-8" }).trim();
    return seed || undefined;
  } catch (error) {
    console.warn(
      `failed to read ${label} seed file '${trimmed}' on pod '${pod}': ${(error as Error).message}`,
    );
    return undefined;
  }
}

function listPods(namespace: string, label: string): string[] {
  const cmd = `kubectl get pods -n ${namespace} -l ${label} -o jsonpath='{.items[*].metadata.name}'`;
  try {
    const raw = execSync(cmd, { encoding: "utf-8" }).trim();
    if (!raw) {
      return [];
    }
    return raw.split(/\s+/).filter(Boolean);
  } catch (error) {
    console.warn(
      `failed to list pods for label '${label}': ${(error as Error).message}`,
    );
    return [];
  }
}

function convertSecretsToEnvObject(
  secrets: SecretsByNode,
): Record<string, string> {
  const env: Record<string, string> = {};

  for (const [nodeName, nodeSecrets] of Object.entries(secrets)) {
    const prefix = (nodeSecrets.envPrefix ?? nodeName).toUpperCase();

    for (const [property, suffix] of SEED_ENV_KEYS) {
      const value = nodeSecrets[property];
      if (typeof value === "string" && value) {
        env[`${prefix}_${suffix}`] = value;
      }
    }

    const connectionString = nodeSecrets.postgres?.connectionString;
    if (connectionString) {
      const roleSegment = nodeSecrets.role === "boot" ? "BOOT_" : "NODE_";
      const key = `DB_SYNC_POSTGRES_CONNECTION_STRING_${roleSegment}${prefix}`;
      env[key] = connectionString;
    }
  }

  return env;
}

const formatNodeKey = (pod: string) => pod.replace(/-/g, "_").toUpperCase();

const getPortFromMapping = (host: string, mapping: PortMapping) => {
  const clusterName = host.replace(/-primary$/, "");
  const entry = Object.entries(mapping).find(([name]) =>
    name.startsWith(clusterName),
  );
  if (!entry) {
    return undefined;
  }
  return entry[1];
};

const PREVIEW_ENV_FIELDS = [
  "DB_SYNC_POSTGRES_CONNECTION_STRING",
  "SEED_PHRASE",
  "AURA_SEED_FILE",
  "GRANDPA_SEED_FILE",
  "CROSS_CHAIN_SEED_FILE",
] as const;

function getPreviewSecrets(namespace: string): Record<string, string> {
  const pods = listPreviewValidatorPods(namespace);
  console.log(`processing ${pods.length} preview validator pod(s)`);

  const secrets: SecretsByNode = {};

  for (const pod of pods) {
    const envValues = readPodEnv(namespace, pod, PREVIEW_ENV_FIELDS);
    const validatorId = parseValidatorId(pod);
    if (!validatorId) {
      console.warn(
        `skipping pod '${pod}' because validator id could not be parsed`,
      );
      continue;
    }

    const auraSeed = readSeedFile(
      namespace,
      pod,
      envValues.AURA_SEED_FILE,
      "aura",
    );
    const grandpaSeed = readSeedFile(
      namespace,
      pod,
      envValues.GRANDPA_SEED_FILE,
      "grandpa",
    );
    const crossChainSeed = readSeedFile(
      namespace,
      pod,
      envValues.CROSS_CHAIN_SEED_FILE,
      "cross-chain",
    );
    const seed =
      envValues.SEED_PHRASE?.trim() ||
      auraSeed ||
      grandpaSeed ||
      crossChainSeed;

    const connectionString =
      envValues.DB_SYNC_POSTGRES_CONNECTION_STRING?.trim() ?? "";

    secrets[pod] = {
      seed,
      auraSeed,
      grandpaSeed,
      crossChainSeed,
      role: "authority",
      envPrefix: `MIDNIGHT_NODE_${validatorId}_0`,
      postgres: connectionString
        ? {
            host: "",
            password: "",
            port: "",
            user: "",
            db: "",
            connectionString,
          }
        : undefined,
    };
  }

  const bootPods = listPreviewBootPods(namespace);
  console.log(`processing ${bootPods.length} preview boot pod(s)`);
  for (const pod of bootPods) {
    const envValues = readPodEnv(
      namespace,
      pod,
      ["DB_SYNC_POSTGRES_CONNECTION_STRING"] as const,
    );
    const bootId = parseBootId(pod);
    if (!bootId) {
      console.warn(`skipping boot pod '${pod}' because id could not be parsed`);
      continue;
    }

    const connectionString =
      envValues.DB_SYNC_POSTGRES_CONNECTION_STRING?.trim() ?? "";

    secrets[pod] = {
      role: "boot",
      envPrefix: `MIDNIGHT_NODE_BOOT_${bootId}_0`,
      postgres: connectionString
        ? {
            host: "",
            password: "",
            port: "",
            user: "",
            db: "",
            connectionString,
          }
        : undefined,
    };
  }

  return convertSecretsToEnvObject(secrets);
}

function listPreviewValidatorPods(namespace: string): string[] {
  const cmd = `kubectl get pods -n ${namespace} -o jsonpath='{.items[*].metadata.name}'`;
  try {
    const raw = execSync(cmd, { encoding: "utf-8" }).trim();
    if (!raw) {
      return [];
    }
    return raw
      .split(/\s+/)
      .filter((name) => /midnight-node-validator/i.test(name));
  } catch (error) {
    console.warn(
      `failed to list preview validator pods: ${(error as Error).message}`,
    );
    return [];
  }
}

function parseValidatorId(pod: string): string | undefined {
  const match = pod.match(/validator-(\d+)-0/);
  if (!match) {
    return undefined;
  }
  return match[1].padStart(2, "0");
}

function listPreviewBootPods(namespace: string): string[] {
  const cmd = `kubectl get pods -n ${namespace} -o jsonpath='{.items[*].metadata.name}'`;
  try {
    const raw = execSync(cmd, { encoding: "utf-8" }).trim();
    if (!raw) {
      return [];
    }
    return raw
      .split(/\s+/)
      .filter((name) => /midnight-node-boot/i.test(name));
  } catch (error) {
    console.warn(
      `failed to list preview boot pods: ${(error as Error).message}`,
    );
    return [];
  }
}

function parseBootId(pod: string): string | undefined {
  const match = pod.match(/boot-(\d+)-0/);
  if (!match) {
    return undefined;
  }
  return match[1].padStart(2, "0");
}
