#!/bin/sh
# Reads [workspace.lints.clippy] from Cargo.toml and outputs comma-separated
# clippy flags for Bazel's --@rules_rust//rust/settings:clippy_flags setting.
# Single source of truth: Cargo.toml defines lint policy, this script bridges to Bazel.
set -e

sed -n '/^\[workspace\.lints\.clippy\]/,/^\[/{//d;p;}' Cargo.toml | \
  grep -v '^[[:space:]]*$' | \
  grep -v '^[[:space:]]*#' | \
  sed 's/[[:space:]]*#.*//' | \
  sed 's/\([^[:space:]]*\).*level[[:space:]]*=[[:space:]]*"\([^"]*\)".*priority[[:space:]]*=[[:space:]]*\([0-9]*\).*/\3 \2 \1/' | \
  sort -n | \
  while read -r _prio level name; do
    name=$(echo "$name" | tr '-' '_')
    case "$level" in
      allow) echo "-Aclippy::${name}" ;;
      warn)  echo "-Wclippy::${name}" ;;
      deny)  echo "-Dclippy::${name}" ;;
    esac
  done | \
  tr '\n' ',' | sed 's/,$/\n/'
