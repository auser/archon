# Sprint: holoarch v2 — Phase 1 MVP

**Started**: 2026-03-07
**Goal**: Working `holoarch` binary with `init`, `verify`, and `status` commands
**Status**: COMPLETE

---

## Tasks

### 1. Workspace Scaffolding
- [x] Create root `Cargo.toml` with workspace members (9 crates)
- [x] Create `.gitignore`
- [x] Create all 9 crate `Cargo.toml` files with stub `lib.rs`/`main.rs`
- [x] Verify `cargo check --workspace` passes

### 2. holoarch-core
- [x] `repo_meta.rs` — RepoMeta, RepoRole, RepoType, ContractDecl, CrateClass, ExceptionRef
- [x] `standards_version.rs` — StandardsVersion parse/compare/drift
- [x] `error.rs` — HoloarchError enum
- [x] `paths.rs` — find_repo_meta walk-up, resolve_arch_root
- [x] `profile.rs` — Profile enum with clap ValueEnum
- [x] Unit tests: YAML round-trip (8 tests passing)

### 3. holoarch-policy
- [x] `model.rs` — PolicyFile, PolicyRule, CheckSpec, Severity, RuleCategory
- [x] `builtin.rs` — 6 hardcoded default rules
- [x] `evaluator.rs` — PolicyEvaluator with exception support
- [x] `loader.rs` — load from arch repo policies/*.yaml with builtin fallback
- [x] Unit tests: rule evaluation, exception suppression (4 tests passing)

### 4. holoarch-verify
- [x] `structural.rs` — file/dir existence checks
- [x] `policy_checks.rs` — metadata field validation
- [x] `report.rs` — ConformanceReport with colored output + JSON
- [x] `runner.rs` — VerifyRunner orchestrator

### 5. holoarch-templates
- [x] `renderer.rs` — template rendering with {{var}} substitution
- [x] Template files: archon.repo.yaml, AGENTS.md, CLAUDE.md, architecture.md, development.md
- [x] Unit tests: variable substitution (2 tests passing)

### 6. holoarch-cli
- [x] `app.rs` — clap derive structs for init, verify, status
- [x] `output.rs` — colored output helpers
- [x] `commands/init.rs` — generate archon.repo.yaml + base docs
- [x] `commands/verify.rs` — run conformance checks with --format and --strict
- [x] `commands/status.rs` — read-only summary
- [x] `main.rs` — command dispatch

### 7. Final Verification
- [x] `cargo check --workspace` — passes
- [x] `cargo clippy --workspace -- -D warnings` — clean
- [x] `cargo test --workspace` — 14 tests passing

---

## Next: Phase 2 — Sync + ADR + Exception

- [ ] `holoarch-sync`: manifest loading, SHA-256 hashing, section-managed merge
- [ ] `holoarch-adr`: ADR creation with numbering, exception records
- [ ] CLI commands: sync, adr new/list, exception new/list
