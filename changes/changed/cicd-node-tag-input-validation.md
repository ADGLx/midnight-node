#indexer #ci
# Add input format validation for `node_tag` workflow dispatch parameter

The `build-indexer-images` workflow now validates the `node_tag` input
against `^[a-zA-Z0-9][a-zA-Z0-9._-]*$` before it reaches any shell or
Docker command. Malformed values are rejected with a clear error annotation.
Empty input (the default) skips validation and falls through to the
`NODE_VERSIONS` file as before.

PR: https://github.com/midnightntwrk/midnight-node/pull/951
