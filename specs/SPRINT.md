# Sprint 002: Phase 2 — Sync + ADR + Exception + AI Fill

**Started**: 2026-03-07
**Goal**: Complete the sync engine, ADR/exception management, and AI-driven template filling
**Status**: IN PROGRESS

---

## Tasks

### 1. archon-sync
- [x] `manifest.rs` — SyncManifest, SyncEntry, Ownership model (2 tests)
- [x] `hasher.rs` — SHA-256 file hashing for change detection (3 tests)
- [x] `engine.rs` — sync loop: fully-managed overwrite, section-managed merge, skip unmanaged (5 tests)
- [x] `sections.rs` — parse/replace `<!-- ARCHON:MANAGED:BEGIN/END -->` blocks (6 tests)
- [x] `state.rs` — `.archon/sync-state.yaml` tracking (2 tests)

### 2. archon-adr
- [x] `model.rs` — Adr, AdrStatus, ExceptionRecord types (3 tests)
- [x] `numbering.rs` — scan specs/adrs/ for next number, slugify (5 tests)
- [x] `create.rs` — `adr new` template generation + listing (4 tests)
- [x] `exception.rs` — ExceptionRecord with expiry dates (3 tests)

### 3. archon-ai integration
- [x] `archon-ai` crate — backend detection, API calls
- [x] `doc_selection.rs` — AI-driven optional doc selection during init
- [x] `fill.rs` — AI-driven `<!-- TODO -->` placeholder filling during init
- [x] `merge.rs` — AI-assisted doc merge during sync
- [x] Wire AI fill into init command (`--no-ai` flag, `--arch-root` for context)
- [x] Wire AI merge into sync command (auto-detects backend, merges local edits with upstream)

### 4. CLI commands
- [x] `commands/sync.rs` — `archon sync [--dry-run] [--force] [--arch-root]`
- [x] `commands/adr.rs` — `archon adr new --title "..." [--status proposed]`
- [x] `commands/adr.rs` — `archon adr list`
- [x] `commands/exception.rs` — `archon exception new --rule --reason [--expires]`
- [x] `commands/exception.rs` — `archon exception list`
- [x] Update `app.rs` and `main.rs` for new commands

### 5. Rename: holoarch → archon
- [x] Rename all 10 crate directories
- [x] Update all Cargo.toml files (workspace + 10 crates)
- [x] Update all Rust source imports
- [x] Update CLI binary name, user-facing strings, env vars
- [x] Update README, Justfile, .gitignore, templates
- [x] Managed section markers: `HOLOARCH` → `ARCHON`
- [x] Env var: `ARCHON_ROOT`

### 6. Architecture repo plan
- [x] Draft plan at `specs/plans/architecture-repo-plan.md`

### 7. Verification
- [x] `cargo check --workspace`
- [x] `cargo clippy --workspace -- -D warnings`
- [x] `cargo test --workspace` (51 tests passing)
- [ ] Manual: `archon sync --dry-run` in a test repo
- [ ] Manual: `archon adr new --title "Test decision"`
