"""Helpers for dual crate universe (native std vs wasm no_std)."""

def wasm_aware_crates(crate_names):
    """Returns a select() picking @crates_wasm// (wasm32) or @crates// (native).

    Args:
        crate_names: list of crate names (e.g. ["frame-support", "sp-runtime"])

    Returns:
        A select() expression resolving to the appropriate crate targets.
    """
    return select({
        "//:wasm32": ["@crates_wasm//:" + name for name in crate_names],
        "//conditions:default": ["@crates//:" + name for name in crate_names],
    })

def native_only_crates(crate_names):
    """Returns a select() that includes crates only in native (non-wasm) builds.

    Use for deps behind #[cfg(feature = "std")] that don't exist in @crates_wasm//.

    Args:
        crate_names: list of crate names (e.g. ["sqlx", "tokio"])

    Returns:
        A select() expression resolving to the crate targets or [] for wasm.
    """
    return select({
        "//:wasm32": [],
        "//conditions:default": ["@crates//:" + name for name in crate_names],
    })

def wasm_rustc_flags():
    """Returns --cfg substrate_runtime for wasm builds (needed by #[runtime_interface])."""
    return select({
        "//:wasm32": ["--cfg", "substrate_runtime"],
        "//conditions:default": [],
    })

def wasm_aware_features(std_features, wasm_features = []):
    """Returns a select() picking features based on platform.

    Args:
        std_features: features for native builds (e.g. ["std"])
        wasm_features: features for wasm builds (default: [])

    Returns:
        A select() expression resolving to the appropriate feature list.
    """
    return select({
        "//:wasm32": wasm_features,
        "//conditions:default": std_features,
    })
