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

`Frame::paste` places sub-frame meshes (`frame.meshes`) **before** the parent's own pending geometry (`frame.buffers`). 
This means anything drawn inside a `with_clip` sub-frame is submitted to the GPU before the parent's fills — reversing draw order and letting a later parent fill (e.g. a row background) overwrite the sub-frame content (e.g. a chevron).

Rule: never put filled paths (meshes) inside `with_clip` if a background fill exists in the same parent frame. Use `with_clip` only for text, or draw fills in the parent frame before calling `with_clip`.