# docs

Documentation crate for compile-time doc testing.

## Overview

This crate exists primarily to enable compile-time validation of code examples in documentation. It contains:

- Test utilities for documentation validation
- Integration tests that verify doc examples compile

## Documentation Resources

The actual documentation files are located in this directory (not as Rust code):

| File | Description |
|------|-------------|
| [`chain_specs.md`](chain_specs.md) | Chain specification documentation |
| [`weights.md`](weights.md) | Runtime weights documentation |
| [`rust-setup.md`](rust-setup.md) | Development environment setup |
| [`development-workflow.md`](development-workflow.md) | Git workflow and contribution guide |
| [`actionlint-guide.md`](actionlint-guide.md) | GitHub Actions linting guide |
| [`fork-testing.md`](fork-testing.md) | Hard fork testing procedures |

### Subdirectories

| Directory | Description |
|-----------|-------------|
| [`decisions/`](decisions/) | Architecture Decision Records (ADRs) |
| [`proposals/`](proposals/) | Design proposals |
| [`signatures/`](signatures/) | GPG signatures for releases |

## Testing

```bash
# Run documentation tests
cargo test -p docs
```

## Related Documentation

- [CONTRIBUTING.md](../CONTRIBUTING.md) - Contribution guidelines
- [README.md](../README.md) - Project overview
- [CHANGELOG.md](../CHANGELOG.md) - Version history

## See Also

- [decisions/](decisions/) - ADR documents
- [proposals/](proposals/) - Design proposals

