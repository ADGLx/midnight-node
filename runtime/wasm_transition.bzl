"""Platform transition for building the WASM runtime on wasm32v1-none."""

def _wasm_transition_impl(settings, attr):
    return {
        "//command_line_option:platforms": str(Label("//:wasm32v1-none")),
        # Clear host-only linker flags (e.g. -fuse-ld=lld from .bazelrc)
        # that rust-lld doesn't understand.
        "@rules_rust//:extra_rustc_flags": [],
    }

_wasm_transition = transition(
    implementation = _wasm_transition_impl,
    inputs = [],
    outputs = [
        "//command_line_option:platforms",
        "@rules_rust//:extra_rustc_flags",
    ],
)

def _transitioned_wasm_impl(ctx):
    files = ctx.attr.wasm_lib[0][DefaultInfo].files
    return [DefaultInfo(files = files)]

transitioned_wasm = rule(
    implementation = _transitioned_wasm_impl,
    attrs = {
        "wasm_lib": attr.label(
            cfg = _wasm_transition,
            mandatory = True,
        ),
        "_allowlist_function_transition": attr.label(
            default = "@bazel_tools//tools/allowlists/function_transition_allowlist",
        ),
    },
)
