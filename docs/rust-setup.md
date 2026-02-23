---
title: Installation
---

## Option A: Manual Installation (Default)

By default, the `.envrc` does **not** activate a Nix devshell. You'll need to install dependencies manually.

### Prerequisites

Midnight-node is built with the Rust programming language on top of Polkadot SDK.

For detailed installation instructions for Rust and Polkadot SDK dependencies, please refer to the official Polkadot SDK documentation:

**[Install Polkadot SDK Dependencies](https://docs.polkadot.com/develop/parachains/install-polkadot-sdk/)**

This guide covers all the necessary build dependencies for different operating systems (Ubuntu, macOS, Windows via WSL, etc.).

### Rust Toolchain

This repository includes a `rust-toolchain.toml` file that specifies the exact Rust version to use. The toolchain will be automatically installed when you run any `cargo` command.

To verify your Rust installation:

```bash
rustup show
```

### Direnv (Optional)

The repository includes an `.envrc` file for environment configuration. You can use direnv to automatically load environment variables:

```bash
# Install direnv
# macOS:
brew install direnv

# Ubuntu/Debian:
sudo apt install direnv

# Add to your shell (~/.bashrc or ~/.zshrc)
eval "$(direnv hook bash)"  # or zsh, fish, etc.

# Allow direnv in the repository
cd /path/to/midnight-node
direnv allow
```

**Manual alternative:** If you don't want to use direnv, source `.envrc` manually before running commands:

```bash
source .envrc
cargo check
cargo test
```

## Option B: Nix Devshell (USE_FLAKE)

The repository includes a `flake.nix` that provides a complete, reproducible development environment with all required tools (Rust toolchain, earthly, clang, nodejs, cosign, etc.).

To enable the Nix devshell, set `USE_FLAKE=1` in your `.envrc.local`:

```bash
echo 'export USE_FLAKE=1' >> /path/to/midnight-node/.envrc.local
direnv allow
```

### Prerequisites

- [Nix](https://docs.determinate.systems/ds-nix/how-to/install/) (the Determinate Nix Installer is recommended — it enables flakes by default)
- [direnv](https://direnv.net/) with [nix-direnv](https://github.com/nix-community/nix-direnv) (recommended)

On first entry, Nix will download and build all dependencies. Subsequent entries are instant (cached). Run `devshell-info` at any time to see what's included.

Two devshells are available:

| Devshell | Command | Use case |
|----------|---------|----------|
| `default` | `nix develop` or `direnv allow` in root | Rust development |
| `local-environment` | `direnv allow` in `local-environment/` | Docker/compose local testing (includes npm deps) |

### Manual alternative (without direnv)

```bash
nix develop                      # Enter default devshell
nix develop .#local-environment  # Enter local-environment devshell
```

## Verify Setup

After completing the setup, verify everything works:

```bash
# Check cargo commands work
cargo check

# Check earthly is available
earthly --version
```
