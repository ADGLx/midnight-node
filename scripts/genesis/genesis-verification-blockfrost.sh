#!/usr/bin/env bash
# This file is part of midnight-node.
# Copyright (C) Midnight Foundation
# SPDX-License-Identifier: Apache-2.0

# Genesis verification using the Blockfrost API instead of Cardano db-sync.
#
# Covers:
#   Step 0  — Cardano tip finalization
#   Step 1  — ICS and Reserve UTxO verification
#   Step 4  — Auth script verification for all upgradable contracts
#
# Steps 2, 3, 5, 6 operate on local files only — no db-sync or Blockfrost needed.
# Run ./genesis-verification.sh for those steps.
#
# Requirements: bash, curl, jq, python3 (stdlib only — no extra packages)
# Usage:
#   BF_PROJECT_ID=mainnet... ./genesis-verification-blockfrost.sh
#   or run without the env var and enter it at the prompt.

set -euo pipefail

RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'
BLUE='\033[0;34m'; CYAN='\033[0;36m'; MAGENTA='\033[0;35m'; NC='\033[0m'; BOLD='\033[1m'; DIM='\033[2m'

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
RES_DIR="$REPO_ROOT/res/mainnet"
BF_BASE="https://cardano-mainnet.blockfrost.io/api/v0"

# "main" in hex — the NFT asset name used inside every governance contract
MAIN_HEX="6d61696e"
# "NIGHT" in hex — the NIGHT token asset name
NIGHT_HEX="4e49474854"

# ---------------------------------------------------------------------------
# Print helpers
# ---------------------------------------------------------------------------
print_header()   { echo -e "\n${BOLD}${BLUE}==================================================================${NC}"; echo -e "${BOLD}${BLUE}  $1${NC}"; echo -e "${BOLD}${BLUE}==================================================================${NC}\n"; }
print_step()     { echo -e "\n${BOLD}${CYAN}>>> $1${NC}\n"; }
print_substep()  { echo -e "\n${BOLD}${CYAN}  ── $1${NC}"; }
print_success()  { echo -e "${GREEN}[PASS]${NC} $1"; }
print_error()    { echo -e "${RED}[FAIL]${NC} $1"; }
print_info()     { echo -e "${BLUE}[INFO]${NC} $1"; }
print_warn()     { echo -e "${YELLOW}[WARN]${NC} $1"; }

# Show the Blockfrost API call about to be made
print_query() {
    echo -e "  ${DIM}GET ${BF_BASE}${1}${NC}"
}

# Show a labelled field with expected and actual values
print_field() {
    local label="$1" value="$2"
    printf "  %-18s %s\n" "$label" "$value"
}

print_expected() { printf "  %-18s %s\n" "expected:"  "$1"; }
print_actual()   { printf "  %-18s %s\n" "actual:"    "$1"; }
print_separator() { echo -e "  ${DIM}──────────────────────────────────────────────────────${NC}"; }

# ---------------------------------------------------------------------------
# Blockfrost helper — returns response body or "null" on 404/error
# ---------------------------------------------------------------------------
bf() {
    local path="$1"
    local body http_code
    http_code=$(curl -s -o /tmp/bf_body.json -w "%{http_code}" \
        -H "project_id: $BF_PROJECT_ID" \
        "${BF_BASE}${path}" 2>/dev/null) || true
    body=$(cat /tmp/bf_body.json 2>/dev/null || echo "null")
    if [[ "$http_code" == "200" ]]; then
        echo "$body"
    elif [[ "$http_code" == "404" ]]; then
        echo "null"
    else
        print_warn "Blockfrost ${path} returned HTTP ${http_code}" >&2
        echo "null"
    fi
}

# ---------------------------------------------------------------------------
# Crypto helpers — both use only Python 3 stdlib
# ---------------------------------------------------------------------------

# Compute blake2b_224(0x03 || hex_bytes) — the Plutus V3 script hash formula
plutus_v3_hash() {
    python3 - "$1" <<'PYEOF'
import hashlib, sys
data = bytes.fromhex(sys.argv[1])
print(hashlib.blake2b(b'\x03' + data, digest_size=28).hexdigest())
PYEOF
}

# Return "yes" if needle bytes appear anywhere inside haystack bytes
bytes_embedded() {
    python3 - "$1" "$2" <<'PYEOF'
import sys
haystack, needle = bytes.fromhex(sys.argv[1]), bytes.fromhex(sys.argv[2])
print("yes" if needle in haystack else "no")
PYEOF
}

# ---------------------------------------------------------------------------
# Datum helpers
# ---------------------------------------------------------------------------

# Extract the raw datum string from a UTxO object.
# Tries inline_datum first; falls back to fetching via data_hash.
get_raw_datum() {
    local utxo_json="$1"
    local inline
    inline=$(echo "$utxo_json" | jq -r '.inline_datum // empty')
    if [[ -n "$inline" && "$inline" != "null" ]]; then
        echo "$inline"
        return 0
    fi
    local dhash
    dhash=$(echo "$utxo_json" | jq -r '.data_hash // empty')
    if [[ -n "$dhash" && "$dhash" != "null" ]]; then
        bf "/scripts/datum/$dhash" | jq -r '.json_value // empty'
    fi
}

# Extract the auth script hash (bytes at list index 2) from a Plutus datum.
# Handles two formats Blockfrost can return:
#   1. Decoded JSON: {"list": [{"bytes":"..."}, {"bytes":""}, {"bytes":"<hash>"}]}
#   2. Raw CBOR hex: 9f581c<28 bytes>40581c<28 bytes>ff  (indefinite array)
extract_auth_from_datum() {
    python3 - "$1" <<'PYEOF'
import sys, json

def decode_cbor_bytes_array(hex_str):
    data = bytes.fromhex(hex_str.strip().strip('"'))
    pos = 0
    b = data[pos]; pos += 1
    indefinite = (b == 0x9f)
    if not indefinite and (b >> 5) != 4:
        return None
    items = []
    while pos < len(data):
        if indefinite and data[pos] == 0xff:
            break
        b = data[pos]; pos += 1
        major = b >> 5; info = b & 0x1f
        if major == 2:  # bytestring
            if info <= 23:   length = info
            elif info == 24: length = data[pos]; pos += 1
            elif info == 25: length = (data[pos] << 8) | data[pos+1]; pos += 2
            else: break
            items.append(data[pos:pos + length].hex())
            pos += length
        else:
            # Non-bytestring element — stop here; we only need up to index 2
            break
    return items if items else None

datum = sys.argv[1] if len(sys.argv) > 1 else ""
if not datum or datum in ("null", "empty", ""):
    sys.exit(0)

# Try decoded JSON (Blockfrost decoded Plutus Data format)
try:
    d = json.loads(datum)
    if isinstance(d, dict) and "list" in d and len(d["list"]) > 2:
        print(d["list"][2].get("bytes", ""))
        sys.exit(0)
except (json.JSONDecodeError, TypeError, AttributeError):
    pass

# Try raw CBOR hex
try:
    items = decode_cbor_bytes_array(datum)
    if items and len(items) > 2:
        print(items[2])
except Exception:
    pass
PYEOF
}

# Strip leading "0x" from a hex string if present
strip_0x() { echo "${1#0x}"; }

# ===========================================================================
# STEP 0: Cardano Tip Finalization
# ===========================================================================
run_step0() {
    print_step "Step 0: Cardano Tip Finalization"
    echo "  What this checks:"
    echo "  The genesis was anchored to a specific Cardano block. This step asks"
    echo "  Blockfrost how many blocks have been built on top of it. If that number"
    echo "  is at least the security_parameter (2160 for mainnet, ~12 hours of blocks)"
    echo "  the block is considered permanently final and cannot be rolled back."
    echo ""

    local tip security_param
    tip=$(strip_0x "$(jq -r '.cardano_tip' "$RES_DIR/cardano-tip.json")")
    security_param=$(jq -r '.cardano.security_parameter' "$RES_DIR/pc-chain-config.json")

    print_field "source file:"   "res/mainnet/cardano-tip.json  →  cardano_tip"
    print_field "block hash:"    "${tip:0:16}...${tip: -8}"
    print_field "source file:"   "res/mainnet/pc-chain-config.json  →  cardano.security_parameter"
    print_field "need conf. ≥:"  "$security_param"
    echo ""

    local bf_path="/blocks/$tip"
    print_query "$bf_path"
    echo "  Checking response fields:  .height  .confirmations"
    echo ""

    local block
    block=$(bf "$bf_path")
    if [[ "$block" == "null" ]]; then
        print_error "Block not found — check the block hash and that your project_id is for mainnet"
        return 1
    fi

    local height confirmations
    height=$(echo "$block" | jq -r '.height')
    confirmations=$(echo "$block" | jq -r '.confirmations')

    print_separator
    print_field "height:"        "$height"
    print_field "confirmations:" "$confirmations"
    print_field "required:"      ">= $security_param"
    print_separator
    echo ""

    if python3 -c "import sys; sys.exit(0 if int('$confirmations') >= int('$security_param') else 1)"; then
        print_success "Step 0: Block $height has $confirmations confirmations — finalized"
        return 0
    else
        print_error "Step 0: Block $height has only $confirmations/$security_param confirmations — not yet finalized"
        return 1
    fi
}

# ===========================================================================
# STEP 1: UTxO Verification (ICS and Reserve)
# ===========================================================================
verify_night_utxos() {
    local label="$1"
    local address="$2"
    local night_policy_raw="$3"
    local config_file="$4"

    local night_policy night_asset
    night_policy=$(strip_0x "$night_policy_raw")
    night_asset="${night_policy}${NIGHT_HEX}"

    echo "  What this checks:"
    echo "  Queries Blockfrost for all unspent NIGHT token outputs at the $label"
    echo "  validator address. Each UTxO is matched against the expected tx hash,"
    echo "  output index, and token quantity recorded in the genesis config file."
    echo ""
    print_field "source file:"   "res/mainnet/$(basename "$config_file")"
    print_field "address:"       "${address:0:20}...${address: -8}"
    print_field "NIGHT asset:"   "${night_asset:0:24}...  (policy_id + hex(\"NIGHT\"))"
    echo ""

    local bf_path="/addresses/$address/utxos/$night_asset"
    print_query "$bf_path"
    echo "  Checking response fields:  [].tx_hash  [].output_index  [].amount[].quantity"
    echo ""

    local bf_utxos
    bf_utxos=$(bf "$bf_path")
    if [[ "$bf_utxos" == "null" ]]; then
        print_error "$label: address not found or holds no NIGHT tokens"
        return 1
    fi

    local all_ok=true

    while IFS= read -r expected_utxo; do
        local tx_hash out_idx exp_amount
        tx_hash=$(echo "$expected_utxo" | jq -r '.tx_hash')
        out_idx=$(echo "$expected_utxo" | jq -r '.output_index')
        exp_amount=$(echo "$expected_utxo" | jq -r '.amount | tostring')

        print_separator
        print_field "tx hash:"    "${tx_hash:0:24}...#${out_idx}"

        local matched
        matched=$(echo "$bf_utxos" | jq \
            --arg tx "$tx_hash" --argjson idx "$out_idx" \
            '[.[] | select(.tx_hash == $tx and .output_index == $idx)] | first // empty')

        if [[ -z "$matched" || "$matched" == "empty" ]]; then
            print_expected "tx $tx_hash#$out_idx  (exists + unspent)"
            print_actual   "not found on Blockfrost"
            print_separator
            print_error "$label UTxO not found on Blockfrost"
            all_ok=false
            continue
        fi

        local actual_amount
        actual_amount=$(echo "$matched" | jq -r \
            --arg asset "$night_asset" \
            '.amount[] | select(.unit == $asset) | .quantity')

        print_expected "$exp_amount NIGHT tokens"
        print_actual   "$actual_amount NIGHT tokens"
        print_separator

        if [[ "$exp_amount" == "$actual_amount" ]]; then
            print_success "$label UTxO ${tx_hash:0:16}...#$out_idx — amount matches"
        else
            print_error "$label UTxO ${tx_hash:0:16}...#$out_idx — amount mismatch"
            all_ok=false
        fi
    done < <(jq -c '.utxos[]' "$config_file")

    local expected_count actual_count
    expected_count=$(jq '.utxos | length' "$config_file")
    actual_count=$(echo "$bf_utxos" | jq 'length')
    if [[ "$expected_count" != "$actual_count" ]]; then
        print_warn "$label: $actual_count UTxO(s) at address now vs $expected_count at genesis (extra may have been added later)"
    fi

    [[ "$all_ok" == "true" ]]
}

run_step1() {
    print_step "Step 1: UTxO Verification (ICS and Reserve)"

    local all_ok=true

    print_substep "ICS (Illiquid Circulation Supply) validator"
    verify_night_utxos "ICS" \
        "$(jq -r '.illiquid_circulation_supply_validator_address' "$RES_DIR/ics-addresses.json")" \
        "$(jq -r '.asset.policy_id' "$RES_DIR/ics-addresses.json")" \
        "$RES_DIR/ics-config.json" || all_ok=false

    echo ""
    print_substep "Reserve validator"
    verify_night_utxos "Reserve" \
        "$(jq -r '.reserve_validator_address' "$RES_DIR/reserve-addresses.json")" \
        "$(jq -r '.asset.policy_id' "$RES_DIR/reserve-addresses.json")" \
        "$RES_DIR/reserve-config.json" || all_ok=false

    echo ""
    if [[ "$all_ok" == "true" ]]; then
        print_success "Step 1: UTxO verification passed!"
    else
        print_error "Step 1: UTxO verification failed"
    fi
    [[ "$all_ok" == "true" ]]
}

# ===========================================================================
# STEP 4: Auth Script Verification
# For each upgradable contract this runs three checks:
#   4a  blake2b_224(0x03 || compiled_code) == policy_id  [local, no API]
#   4b  two_stage_policy_id is embedded in compiled_code  [local, no API]
#   4c  governance NFT datum on Cardano contains expected auth script hash
# ===========================================================================
verify_contract_auth() {
    local label="$1"
    local policy_id="$2"
    local compiled_code="$3"
    local two_stage_id="$4"
    local expected_auth="$5"

    local code_len=$(( ${#compiled_code} / 2 ))
    local all_ok=true

    # ---- 4a: Policy hash (local computation, no API) -------------------------
    echo ""
    echo "  Check 4a — compiled_code integrity  [local, no API call]"
    echo "  Verifies the contract code has not been tampered with. The policy_id"
    echo "  must equal blake2b_224(0x03 || compiled_code_bytes) — the standard"
    echo "  Plutus V3 script hash formula."
    echo ""
    print_field "method:"       "blake2b_224( 0x03 || compiled_code )"
    print_field "code size:"    "$code_len bytes"
    print_field "expected:"     "$policy_id"

    local computed
    computed=$(plutus_v3_hash "$compiled_code")
    print_field "computed:"     "$computed"
    print_separator

    if [[ "$computed" == "$policy_id" ]]; then
        print_success "4a [$label] policy_id = blake2b_224(0x03 || compiled_code)"
    else
        print_error   "4a [$label] policy_id mismatch — compiled_code may have been altered"
        all_ok=false
    fi

    # ---- 4b: two_stage embedded (local byte search, no API) ------------------
    echo ""
    echo "  Check 4b — governance policy embedded in code  [local, no API call]"
    echo "  The two_stage_policy_id must appear as a raw byte sequence inside the"
    echo "  compiled contract code. This links the contract to its governance policy."
    echo ""
    print_field "method:"       "byte search: two_stage_policy_id in compiled_code"
    print_field "searching for:" "$two_stage_id"
    print_field "in code:"      "${compiled_code:0:24}... ($code_len bytes)"
    print_separator

    if [[ "$(bytes_embedded "$compiled_code" "$two_stage_id")" == "yes" ]]; then
        print_success "4b [$label] two_stage_policy_id is embedded in compiled_code"
    else
        print_error   "4b [$label] two_stage_policy_id NOT found in compiled_code"
        all_ok=false
    fi

    # ---- 4c: Auth script from Cardano (Blockfrost) ---------------------------
    echo ""
    echo "  Check 4c — authorization script on Cardano  [Blockfrost API]"
    echo "  Each contract holds a governance NFT (the 'main' token). That UTxO's"
    echo "  datum encodes the authorization script hash at list position [2]."
    echo "  All four contracts must reference the same authorization script."
    echo ""

    local main_asset="${two_stage_id}${MAIN_HEX}"
    print_field "governance NFT:" "${main_asset:0:24}...  (two_stage_policy_id + hex(\"main\"))"
    echo ""

    # API call 1: find where the governance NFT lives
    local bf_path1="/assets/$main_asset/addresses"
    print_query "$bf_path1"
    echo "  Checking response fields:  [0].address"

    local locations token_addr
    locations=$(bf "$bf_path1")
    token_addr=$(echo "$locations" | jq -r '.[0].address // empty' 2>/dev/null || echo "")

    if [[ -z "$token_addr" || "$token_addr" == "null" ]]; then
        print_actual   "token not found on Cardano"
        print_separator
        print_error "4c [$label] governance NFT not found — contract may not be deployed"
        all_ok=false
        return 1
    fi

    print_field "token lives at:" "${token_addr:0:20}...${token_addr: -8}"
    echo ""

    # API call 2: get the UTxO (with datum) at that address
    local bf_path2="/addresses/$token_addr/utxos/$main_asset"
    print_query "$bf_path2"
    echo "  Checking response fields:  [0].inline_datum  (CBOR list, index [2] = auth script hash)"
    echo ""

    local utxos utxo raw_datum observed
    utxos=$(bf "$bf_path2")
    utxo=$(echo "$utxos" | jq '.[0] // empty')

    local tx_hash_short
    tx_hash_short=$(echo "$utxo" | jq -r '.tx_hash // "unknown"' | cut -c1-16)
    local out_idx
    out_idx=$(echo "$utxo" | jq -r '.output_index // "?"')
    print_field "UTxO:"         "${tx_hash_short}...#${out_idx}"

    raw_datum=$(get_raw_datum "$utxo")
    observed=$(extract_auth_from_datum "$raw_datum")

    print_separator
    print_expected "$expected_auth"
    print_actual   "${observed:-<could not decode datum>}"
    print_separator

    if [[ -z "$observed" ]]; then
        print_error "4c [$label] could not decode auth script from datum"
        all_ok=false
    elif [[ "$observed" == "$expected_auth" ]]; then
        print_success "4c [$label] auth script on Cardano matches"
    else
        print_error   "4c [$label] auth script mismatch — contract may use a different governance policy"
        all_ok=false
    fi

    [[ "$all_ok" == "true" ]]
}

run_step4() {
    print_step "Step 4: Auth Script Verification"
    echo "  What this checks:"
    echo "  Every upgradable Midnight contract (ICS, Reserve, Federated Authority,"
    echo "  Permissioned Candidates) must use the same authorization script — the"
    echo "  governance policy that controls future upgrades. Three sub-checks run"
    echo "  per contract: code integrity (local), governance policy embedded (local),"
    echo "  and observed auth script on Cardano (Blockfrost)."
    echo ""

    local auth_policy
    auth_policy=$(jq -r '.authorization_policy_id' "$RES_DIR/authorization-addresses.json")
    print_field "source file:"    "res/mainnet/authorization-addresses.json"
    print_field "expected auth:"  "$auth_policy"

    local all_ok=true

    print_substep "ICS (Illiquid Circulation Supply)"
    verify_contract_auth "ICS" \
        "$(jq -r '.illiquid_circulation_supply_validator_policy_id' "$RES_DIR/ics-addresses.json")" \
        "$(jq -r '.illiquid_circulation_supply_validator_compiled_code' "$RES_DIR/ics-addresses.json")" \
        "$(jq -r '.illiquid_circulation_supply_validator_two_stage_policy_id' "$RES_DIR/ics-addresses.json")" \
        "$auth_policy" || all_ok=false

    print_substep "Reserve"
    verify_contract_auth "Reserve" \
        "$(jq -r '.reserve_validator_policy_id' "$RES_DIR/reserve-addresses.json")" \
        "$(jq -r '.reserve_validator_compiled_code' "$RES_DIR/reserve-addresses.json")" \
        "$(jq -r '.reserve_validator_two_stage_policy_id' "$RES_DIR/reserve-addresses.json")" \
        "$auth_policy" || all_ok=false

    print_substep "Federated Authority — Council"
    verify_contract_auth "FedAuth/Council" \
        "$(jq -r '.council_policy_id' "$RES_DIR/federated-authority-addresses.json")" \
        "$(jq -r '.council_compiled_code' "$RES_DIR/federated-authority-addresses.json")" \
        "$(jq -r '.council_two_stage_policy_id' "$RES_DIR/federated-authority-addresses.json")" \
        "$auth_policy" || all_ok=false

    print_substep "Federated Authority — Technical Committee"
    verify_contract_auth "FedAuth/TechCommittee" \
        "$(jq -r '.technical_committee_policy_id' "$RES_DIR/federated-authority-addresses.json")" \
        "$(jq -r '.technical_committee_compiled_code' "$RES_DIR/federated-authority-addresses.json")" \
        "$(jq -r '.technical_committee_two_stage_policy_id' "$RES_DIR/federated-authority-addresses.json")" \
        "$auth_policy" || all_ok=false

    print_substep "Permissioned Candidates"
    verify_contract_auth "PermCandidates" \
        "$(jq -r '.permissioned_candidates_policy_id' "$RES_DIR/permissioned-candidates-addresses.json")" \
        "$(jq -r '.permissioned_candidates_compiled_code' "$RES_DIR/permissioned-candidates-addresses.json")" \
        "$(jq -r '.permissioned_candidates_two_stage_policy_id' "$RES_DIR/permissioned-candidates-addresses.json")" \
        "$auth_policy" || all_ok=false

    echo ""
    if [[ "$all_ok" == "true" ]]; then
        print_success "Step 4: All auth script checks passed!"
    else
        print_error "Step 4: Some auth script checks failed"
    fi
    [[ "$all_ok" == "true" ]]
}

# ===========================================================================
# Main
# ===========================================================================
main() {
    print_header "Midnight Genesis Verification — Blockfrost"

    echo "  This script verifies the Midnight mainnet genesis chain specification"
    echo "  using the Blockfrost public Cardano API — no db-sync required."
    echo ""
    echo "  Covered by this script (requires live Cardano data):"
    echo "    Step 0 — Cardano tip finalization"
    echo "    Step 1 — NIGHT token UTxO amounts at ICS and Reserve validators"
    echo "    Step 4 — Authorization scripts for all 5 upgradable contracts"
    echo ""
    echo "  Covered by ./genesis-verification.sh (local files only, no db-sync):"
    echo "    Step 2 — LedgerState: supply invariant, parameters, genesis timestamp"
    echo "    Step 3 — Dparameter: validator set consistency"
    echo "    Step 5 — Genesis message embedded in chain spec"
    echo "    Step 6 — Genesis timestamp matches Cardano anchor block"
    echo ""

    # Dependency check
    local missing=()
    for dep in curl jq python3; do
        command -v "$dep" &>/dev/null || missing+=("$dep")
    done
    if [[ ${#missing[@]} -gt 0 ]]; then
        print_error "Missing required tools: ${missing[*]}"
        exit 1
    fi

    # Blockfrost project ID
    if [[ -z "${BF_PROJECT_ID:-}" ]]; then
        echo -en "${BOLD}Blockfrost project_id${NC} (sign up free at blockfrost.io, create a Cardano mainnet project): " >&2
        read -r BF_PROJECT_ID
        export BF_PROJECT_ID
    fi

    # Verify API connectivity
    print_query "/health"
    local health
    health=$(bf "/health")
    if [[ "$health" == "null" ]]; then
        print_error "Blockfrost API unreachable — verify your project_id and internet connection"
        exit 1
    fi
    print_info "Blockfrost API connection: OK"
    echo ""

    local s0=fail s1=fail s4=fail overall=true

    run_step0 && s0=pass || overall=false
    run_step1 && s1=pass || overall=false
    run_step4 && s4=pass || overall=false

    print_header "Verification Summary"

    local pass_sym="${GREEN}[PASS]${NC}" fail_sym="${RED}[FAIL]${NC}" skip_sym="${YELLOW}[SKIP]${NC}"
    [[ "$s0" == pass ]] && echo -e "  $pass_sym Step 0: Cardano Tip Finalization" || echo -e "  $fail_sym Step 0: Cardano Tip Finalization"
    [[ "$s1" == pass ]] && echo -e "  $pass_sym Step 1: UTxO Verification (ICS + Reserve)" || echo -e "  $fail_sym Step 1: UTxO Verification (ICS + Reserve)"
    echo -e "  $skip_sym Step 2: LedgerState Verification     (local files — run ./genesis-verification.sh)"
    echo -e "  $skip_sym Step 3: Dparameter Verification      (local files — run ./genesis-verification.sh)"
    [[ "$s4" == pass ]] && echo -e "  $pass_sym Step 4: Auth Script Verification" || echo -e "  $fail_sym Step 4: Auth Script Verification"
    echo -e "  $skip_sym Step 5: Genesis Message Verification (local files — run ./genesis-verification.sh)"
    echo -e "  $skip_sym Step 6: Genesis Timestamp Verification (local files — run ./genesis-verification.sh)"
    echo ""

    if [[ "$overall" == "true" ]]; then
        print_success "All Blockfrost checks passed!"
        exit 0
    else
        print_error "Some checks failed — see details above."
        exit 1
    fi
}

main "$@"
