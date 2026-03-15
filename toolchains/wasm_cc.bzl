"""CC toolchain for wasm32 cross-compilation.

Provides clang --target=wasm32-unknown-unknown for crates that need C code
compiled (e.g. secp256k1-sys). Uses the host clang's wasm32 backend.
"""

load("@bazel_tools//tools/cpp:cc_toolchain_config_lib.bzl", "tool_path")
load("@rules_cc//cc:defs.bzl", "CcToolchainConfigInfo")

def _wasm_cc_toolchain_config_impl(ctx):
    tool_paths = [
        tool_path(name = "gcc", path = "wasm32_cc.sh"),
        tool_path(name = "ld", path = "noop_cc.sh"),
        tool_path(name = "ar", path = "wasm32_ar.sh"),
        tool_path(name = "cpp", path = "wasm32_cc.sh"),
        tool_path(name = "gcov", path = "noop_cc.sh"),
        tool_path(name = "nm", path = "noop_cc.sh"),
        tool_path(name = "objdump", path = "noop_cc.sh"),
        tool_path(name = "strip", path = "noop_cc.sh"),
    ]
    return cc_common.create_cc_toolchain_config_info(
        ctx = ctx,
        toolchain_identifier = "wasm32-none",
        host_system_name = "local",
        target_system_name = "wasm32-unknown-none",
        target_cpu = "wasm32",
        target_libc = "unknown",
        compiler = "clang",
        abi_version = "unknown",
        abi_libc_version = "unknown",
        tool_paths = tool_paths,
    )

wasm_cc_toolchain_config = rule(
    implementation = _wasm_cc_toolchain_config_impl,
    provides = [CcToolchainConfigInfo],
)
