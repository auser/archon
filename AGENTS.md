# AGENTS.md

## Rust Development Standards

### Code Quality

- **Zero warnings policy**: All code must compile with `cargo clippy --all-targets -- -D warnings` producing zero warnings and zero errors. Fix clippy lints immediately — do not suppress them with `#[allow(...)]` unless there is a documented, justified reason.
- Run `cargo fmt` before committing. All code must conform to the project's `rustfmt` configuration.
- Run `cargo test` to ensure no regressions before finalizing changes.

### Design Principles

- **Prefer traits over concrete types**: Define behavior through traits to enable polymorphism, testability, and extensibility. Use trait objects (`dyn Trait`) for runtime polymorphism and generics (`impl Trait` / `<T: Trait>`) for zero-cost compile-time dispatch.
- **Use macros to reduce boilerplate**: When you see repeated patterns across the codebase (e.g., similar struct implementations, repetitive match arms, common error handling), extract them into declarative (`macro_rules!`) or procedural macros. Macros should make code more readable, not less — if a macro is harder to understand than the code it replaces, don't use it.
- **Leverage the type system**: Use newtypes, enums, and phantom types to encode invariants at compile time. Prefer `enum` over stringly-typed fields. Use `Option` and `Result` idiomatically — avoid unwrap in library code.
- **Error handling**: Use `anyhow::Result` for application-level errors. Use `thiserror` for library-level error types when callers need to match on variants. Always provide context with `.context()` or `.with_context()`.
- **Ownership and borrowing**: Prefer borrowing (`&T`, `&mut T`) over cloning. Clone only when ownership transfer is genuinely needed. Use `Cow<'_, T>` when a function may or may not need to own data.

### Architecture Patterns

- Keep functions small and focused. If a function exceeds ~50 lines, consider breaking it into smaller helpers.
- Prefer composition over inheritance. Use trait composition (`trait A: B + C`) to build complex behaviors from simple ones.
- Use the builder pattern for structs with many optional fields.
- Prefer iterators and combinators (`map`, `filter`, `collect`) over manual loops where they improve clarity.
- Use `impl` blocks to group related methods. Separate public API from internal helpers.

### What Not to Do

- Do not use `unsafe` without a safety comment explaining why it is sound.
- Do not suppress clippy lints without a comment explaining the exception.
- Do not use `.unwrap()` or `.expect()` in non-test code unless the invariant is provably guaranteed and documented.
- Do not add dependencies without justification. Prefer the standard library when it covers the use case.
