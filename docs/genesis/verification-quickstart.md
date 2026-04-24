# Genesis Verification — Quickstart Guide

This guide is for people who need to verify a Midnight genesis chain specification but don't have a Rust development environment set up. It uses a pre-built Docker image so you only need Docker and a terminal.

For the full technical reference, see [verification.md](verification.md).

---

## What is genesis verification?

Before the Midnight network launches, a chain specification file (`chain-spec-raw.json`) is produced that encodes the entire starting state of the blockchain. Anyone can independently verify that this file:

- Matches the state of the Cardano smart contracts it was derived from
- Contains the correct token supply (24 billion NIGHT)
- Has not been tampered with

Running this verification does **not** require you to trust the team that produced the file.

---

## What you need

**Software**

- [ ] [Docker](https://docs.docker.com/get-docker/) installed and running
- [ ] `bash` shell (macOS Terminal, Linux terminal, or WSL on Windows)
- [ ] `jq` — JSON processor ([install instructions](https://jqlang.github.io/jq/download/))

**Access** _(only needed for Steps 0, 1, and 4 — see below)_

- [ ] A Cardano db-sync PostgreSQL connection string  
  _(Ask your node team contact if you don't have one. It looks like: `postgres://user:password@host:5432/cexplorer`)_
- [ ] **Or:** a free [Cardano explorer](#running-without-db-sync-steps-0-1-4) if you want to do those three steps manually instead

**Files** — the `res/mainnet/` directory from the repository, or a zip provided by the team. It must contain:

| File | What it is |
|------|------------|
| `chain-spec-raw.json` | The genesis file being verified |
| `cardano-tip.json` | The Cardano block the genesis was anchored to |
| `cnight-config.json` | cNIGHT bridge configuration |
| `ics-config.json` | ICS treasury configuration |
| `reserve-config.json` | Reserve pool configuration |
| `federated-authority-config.json` | Governance configuration |
| `permissioned-candidates-config.json` | Initial validator set |
| `system-parameters-config.json` | System parameters |
| `ledger-parameters-config.json` | Ledger parameters |
| `message-config.json` | Expected genesis remark message |
| `pc-chain-config.json` | Partner chain configuration |
| `*-addresses.json` | Smart contract addresses (5 files) |

---

## Setup

### 1. Clone or download the repository

If you have `git`:

```bash
git clone https://github.com/midnight-ntwrk/midnight-node.git
cd midnight-node
```

Or download and extract a zip of the repository.

### 2. Get the midnight-node binary via Docker

**Apple Silicon (M1/M2/M3/M4):**

```bash
docker pull midnightntwrk/midnight-node:latest-main-arm64

# Extract the binary to where the verification script expects it
mkdir -p target/release
docker create --platform linux/arm64 --name midnight-tmp midnightntwrk/midnight-node:latest-main-arm64
docker cp midnight-tmp:/midnight-node target/release/midnight-node
docker rm midnight-tmp
chmod +x target/release/midnight-node
```

**Intel Mac / Linux / Windows (WSL):**

```bash
docker pull midnightntwrk/midnight-node:latest-main-amd64

# Extract the binary to where the verification script expects it
mkdir -p target/release
docker create --platform linux/amd64 --name midnight-tmp midnightntwrk/midnight-node:latest-main-amd64
docker cp midnight-tmp:/midnight-node target/release/midnight-node
docker rm midnight-tmp
chmod +x target/release/midnight-node
```

Confirm it works:

```bash
./target/release/midnight-node --version
```

---

## Run the verification

```bash
./scripts/genesis/genesis-verification.sh
```

The script will prompt you for two inputs:

1. **DB Sync PostgreSQL connection string** — your db-sync connection (see prerequisites above)
2. **Cardano block hash (tip)** — pre-filled from `res/mainnet/cardano-tip.json` if it exists; press Enter to accept

After that, it runs each check automatically and asks you to confirm before proceeding to the next step.

### Expected successful output

```
=================================================================
  Verification Summary
=================================================================

Results for mainnet:

  [PASS] Step 0: Cardano Tip Finalization
  [PASS] Step 1: Config File Regeneration
  [PASS] Step 2: LedgerState Verification
  [PASS] Step 3: Dparameter Verification
  [PASS] Step 4: Auth Script Verification
  [PASS] Step 5: Genesis Message Verification
  [PASS] Step 6: Genesis Timestamp Verification

[PASS] All verification checks passed!
```

---

## What each check means (plain English)

| Step | What is being checked | Why it matters |
|------|-----------------------|----------------|
| **0 – Cardano Tip Finalization** | The Cardano block used as the anchor point has enough subsequent blocks built on top of it to be considered permanent. | Prevents the genesis from being anchored to a block that could later be rolled back. |
| **1 – Config File Regeneration** | The configuration files in `res/mainnet/` can be independently regenerated from the live Cardano smart contract state and produce the same result. | Confirms the configs are not fabricated — they reflect what is actually on Cardano. |
| **2 – LedgerState Verification** | The genesis state inside `chain-spec-raw.json` is internally consistent: token supply totals exactly 24 billion NIGHT, the reserve pool and treasury values are correct, and parameters match the config files. | Catches any tampering with token amounts or parameters. |
| **3 – Dparameter Verification** | The initial validator set parameters are consistent (no pre-registered candidates, and the permissioned candidate count matches the actual list). | Ensures the validator set is properly initialized. |
| **4 – Auth Script Verification** | All upgradable Cardano smart contracts use the same authorization (governance) script, and their code hashes match what is recorded on-chain. | Confirms no contract has been silently swapped for a different version. |
| **5 – Genesis Message Verification** | The genesis block contains a specific remark (message) that matches the expected text in `message-config.json`. | Provides a human-readable anchor that can be publicly quoted and independently verified. |
| **6 – Genesis Timestamp Verification** | The timestamp encoded in the genesis block matches the Cardano block timestamp. | Confirms the genesis time was not altered after the Cardano anchor block was chosen. |

---

## Running without db-sync (Steps 0, 1, 4)

Only three of the seven steps query a Cardano db-sync database. The other four — **Steps 2, 3, 5, and 6** — run entirely against the local files and do not need a database connection at all.

### Option A — Use the Blockfrost script (recommended)

A separate script covers Steps 0, 1, and 4 using the [Blockfrost](https://blockfrost.io) API instead of db-sync. You need a free Blockfrost account (sign up at blockfrost.io, create a Cardano mainnet project to get a `project_id`).

The diagram below shows how the two scripts divide the work and what each one depends on:

```mermaid
flowchart LR
    cardano[("Cardano\nMainnet")]
    blockfrost["Blockfrost API\nblockfrost.io"]
    cardano <-->|public REST API| blockfrost

    subgraph local["Your Machine"]
        configs["res/mainnet/\nconfig files"]
        binary["midnight-node binary\n(extracted via Docker)"]

        subgraph bfscript["genesis-verification-blockfrost.sh\n— no binary required —\ncurl · jq · python3"]
            direction TB
            s0["Step 0 — Cardano tip finalization"]
            s1["Step 1 — NIGHT UTxO amounts"]
            s4["Step 4 — Authorization scripts"]
        end

        subgraph mainscript["genesis-verification.sh\n— requires binary —"]
            direction TB
            s2["Step 2 — LedgerState & token supply"]
            s3["Step 3 — Dparameter / validator set"]
            s5["Step 5 — Genesis remark message"]
            s6["Step 6 — Genesis timestamp"]
        end
    end

    blockfrost -->|live chain data| bfscript
    configs --> bfscript
    configs --> mainscript
    binary --> mainscript

    bfscript --> result["All 7 steps ✓ verified"]
    mainscript --> result
```

```bash
BF_PROJECT_ID=mainnetYourProjectIdHere \
    ./scripts/genesis/genesis-verification-blockfrost.sh
```

Requirements: `curl`, `jq`, `python3` (standard library only — no extra packages).

Then run the remaining local steps via the main script, skipping Steps 0, 1, and 4:

```bash
./scripts/genesis/genesis-verification.sh
# When prompted to run Steps 0, 1, 4 — answer n (skip)
```

### Option B — Manual verification with a Cardano explorer

When the main script asks for a PostgreSQL connection string, you can enter a dummy value (e.g. `postgres://localhost/x`) and then **skip** Steps 0, 1, and 4 when prompted.

### Step 0 — manually verify Cardano tip finalization

The step checks that the anchor block has at least 2160 subsequent blocks on top of it (roughly 12 hours of Cardano blocks at ~20 sec/block).

1. Open the `cardano-tip.json` file and copy the `cardano_tip` hash value.
2. Look it up on a public Cardano explorer such as [CardanoScan](https://cardanoscan.io) or [pool.pm](https://pool.pm).
3. Note the block height of the tip block, then note the current chain tip height.
4. If the difference is greater than 2160, the block is finalized.

### Step 1 — manually spot-check config regeneration

This step verifies that the config files in `res/mainnet/` match what is currently on-chain for each Cardano smart contract. A full manual re-derivation requires deep Cardano tooling, but you can spot-check key fields:

- For each `*-addresses.json` file, the `policy_id` values are Cardano script hashes. You can look them up on [CardanoScan](https://cardanoscan.io) or via [Koios](https://koios.rest) to confirm the script exists on-chain at the claimed address.
- The `cardano-tip.json` anchor block hash should appear as a real block on a Cardano explorer.

If you need full automated verification of Step 1, you need either db-sync access or a Koios/Blockfrost API key — contact the node team.

### Step 4 — manually verify auth scripts

Each of the four upgradable contracts has a `policy_id` in its `*-addresses.json` file. This is the Blake2b-224 hash of the contract's compiled code.

1. Open `federated-authority-addresses.json`, `ics-addresses.json`, `permissioned-candidates-addresses.json`, and `reserve-addresses.json`.
2. Note the `policy_id` value in each file — they should all be the same value.
3. Look up any one of the contract addresses on [CardanoScan](https://cardanoscan.io). The script hash shown should match the `policy_id`.

If all four files share the same `policy_id` and it matches what the explorer shows, the authorization script check passes.

---

## Troubleshooting

### "Cannot connect to Docker daemon"

Docker is not running. Open Docker Desktop (macOS/Windows) or run `sudo systemctl start docker` (Linux).

### "docker: permission denied"

On Linux, add your user to the docker group: `sudo usermod -aG docker $USER`, then log out and back in.

### "midnight-node binary not found"

The binary was not extracted correctly. Re-run the Docker extraction commands from the Setup section above.

### Step 0 fails — "Cardano tip is NOT finalized"

The Cardano network has not produced enough blocks after the anchor block yet. Wait 1–2 hours and try again with the same block hash.

### Step 0 fails — database connection error

- Check your connection string is correct
- If connecting locally (non-SSL), the script sets `ALLOW_NON_SSL=true` automatically
- Confirm the PostgreSQL port (default 5432) is reachable: `nc -zv <host> 5432`
- Confirm db-sync has fully synced to the tip block

### Step 1 — config files differ

The generated config does not match what is in the repository. This is a significant finding — note the exact differences shown and report them to the node team.

### Step 4 fails — auth script mismatch

A smart contract's code does not match what was expected. This is a significant finding — do not proceed with network launch and escalate immediately.

---

## Reporting results

After running all steps, take a screenshot of the summary output and share it with your contact on the node team. If any step shows `[FAIL]`, include the full output for that step.