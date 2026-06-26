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

# Canvas / wgpu Rendering Gotcha

`Frame::paste` places sub-frame meshes **before** the parent's own pending buffer. Draw order in the final mesh list is determined by the order `with_clip` closures complete, not by call-site order.

Rule: if a background fill must appear beneath content drawn inside `with_clip`, wrap the background in its own `with_clip` first. That flushes it to a mesh immediately, so subsequent sub-frame meshes (cell content, chevrons) are appended after it.