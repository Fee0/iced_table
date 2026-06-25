# Rust Style Guide

## Module Structure

### No `mod.rs`

We use the modern module style introduced in Rust 2018. A module `foo` is either:

- `foo.rs` (if it has no children)
- `foo.rs` + `foo/` directory (if it has children)

Never `foo/mod.rs`. The old style makes every tab in your editor say `mod.rs` and that's reason enough to avoid it.

### One Semantic Type Per Module

Each module should contain one primary type and its closely related types. "Closely related" means types that exist only
to serve the primary type — enum variants, configuration structs, small helpers.

When in doubt: if a type could conceivably be imported independently by code that doesn't care about the parent type, it
deserves its own module.

---

## Naming

All identifiers must use full words, not abbreviations.

### No Composite Names — Use the Module Path

The module system is a namespace. Use it. Don't flatten hierarchies into names.
If you're writing `FooBar` and `Foo` is a module, it should be `foo::Bar`.

### No Aliased Imports

Never rename imports to avoid conflicts. If two types have the same name, use the module path to disambiguate at the
call site.

### Import Style

Prefer importing the parent module, not individual types, when you're using more than one item from it:

For standard library and well-known crate types (`HashMap`, `Vec`, `Result`, `anyhow::Context`), direct imports are
fine.

---

## Error Handling

### Use `thiserror` for Library Errors

Each subsystem defines its own error type inside a error.rs file using thiserror. Also add a Results type that uses the
error. All API's of the subsystem use this Result type.

### Use `anyhow` at the Binary Level

Binary crates and CLI tools can use `anyhow::Result` for top-level error handling. Library crates within the workspace
should use typed errors.

### No `unwrap()` in Production Code

Use `expect("reason")` if a panic is genuinely the right response (invariant violation). Otherwise, propagate errors.
Tests can use `unwrap()` freely.

---

## Struct Design

### Builder Pattern for Complex Construction

If a struct has more than 3-4 fields, especially optional ones, use a builder:

---

## Code Organization Within a File

1. Module-level doc comment
2. Imports (std, external crates, crate-internal — separated by blank lines)
3. Type definitions (structs, enums)
4. Trait implementations (Display, From, etc.)
5. Inherent implementations (impl Foo { ... })
6. Private helper functions
7. Tests (mod tests { ... })

---

## Edition

All crates use the Rust 2024 edition:

```toml
[package]
name = "my-crate"
edition = "2024"
```

---

## Dependencies

### Be Conservative

Every dependency is a liability. Prefer:

1. Standard library
2. Well-maintained, widely-used crates (`serde`, `tokio`, `toml`, `thiserror`, `anyhow`, `tracing`)
3. Nothing else unless there's a strong reason

---

## Testing

### Test Placement

Unit tests go in the same file as the code they test, in a `#[cfg(test)] mod tests` block. Integration tests go in
`tests/`. If only public APIs are tested, it is an integration test.

### Test Naming

Descriptive, reads like a sentence. No `test_` prefix (the `#[test]` attribute is sufficient).

---

## Documentation

Comments explain **why**, in the present tense, describing the code as it stands now.
Golden Rule: Code should be self-documenting. Comments explain why, never what or how.

### Doc Comments on Public Items

Every public type, function, and module gets a doc comment. Keep it concise — one line if possible, a short paragraph if
needed.

### No Redundant Comments

Don't comment what the code obviously does. Comment *why* when the reason isn't obvious.

### No Process, Phase, or Archaeology Comments

A comment describes the code as it is *now* — never the journey that produced it.

Strip three kinds before you commit:

**Chain-of-thought** — the algorithm narrated back to you. The code already says *what* it does.

**Phase / plan bookkeeping** — The phases were yours during implementation; they mean nothing to the next reader.

**Archaeology / changelog** — comments that describe a *change* rather than the code:

## Code Style

- Nested ifs are unreadable. Use early returns and descriptive variable names.
- CRITICAL PRINCIPLE: Code should live close to where it's used.
- All numeric literals must be named constants with descriptive names
- No unsafe code anywhere
- Core Principle: Code Locality - Keep feature-specific logic together in the same module.