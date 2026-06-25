# Code Style Rules

Read and follow:

- .claude/guides/RUST_STYLE.md

# Development

This project is under development and doesn't care about breaking changes.
Do not keep legacy stuff alive.

# Post-change Checklist

Each command must succeed without warnings or errors.

```bash
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo nextest run --all-features
cargo fmt
```

# Clippy

Do not whitelist warnings in Cargo.toml.