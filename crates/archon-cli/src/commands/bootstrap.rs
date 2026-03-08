use anyhow::{Context, Result};
use colored::Colorize;

use crate::app::BootstrapArgs;
use crate::output;

struct FileToCreate {
    path: &'static str,
    content: String,
}

pub fn run(args: BootstrapArgs) -> Result<()> {
    let target = match &args.path {
        Some(p) => std::path::PathBuf::from(p),
        None => std::env::current_dir().context("getting current directory")?,
    };

    if !target.exists() {
        std::fs::create_dir_all(&target)
            .with_context(|| format!("creating directory {}", target.display()))?;
    }

    let name = target
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("hologram-architecture")
        .to_string();

    let version = &args.standards_version;

    output::print_header(&format!("archon bootstrap: {name}"));
    println!();

    if args.dry_run {
        eprintln!("  {}", "(dry run)".yellow());
    }

    let files = build_file_list(&name, version);

    for file in &files {
        let full_path = target.join(file.path);

        if args.dry_run {
            output::print_dry_run(file.path);
            continue;
        }

        if full_path.exists() && !args.force {
            output::print_skipped(file.path, "already exists, use --force to overwrite");
            continue;
        }

        if let Some(parent) = full_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating directory {}", parent.display()))?;
        }

        std::fs::write(&full_path, &file.content)
            .with_context(|| format!("writing {}", full_path.display()))?;

        output::print_created(file.path);
    }

    if args.dry_run {
        println!("\n  (dry run -- no files written)");
    } else {
        println!();
        eprintln!(
            "  {} Architecture repo bootstrapped at {}",
            "✓".green(),
            target.display()
        );
        eprintln!();
        eprintln!("  Next steps:");
        eprintln!("    1. cd {}", target.display());
        eprintln!("    2. git init  (if not already a repo)");
        eprintln!("    3. Review and edit the initial ADRs in specs/adrs/");
        eprintln!("    4. Edit ecosystem/repos.yaml with your repos");
        eprintln!("    5. Customize templates/ for your managed sections");
        eprintln!(
            "    6. Set ARCHON_ROOT={} in your shell",
            target.display()
        );
    }

    println!();
    Ok(())
}

fn build_file_list(name: &str, version: &str) -> Vec<FileToCreate> {
    vec![
        // ── Root files ──────────────────────────────────────────────
        FileToCreate {
            path: "hologram.repo.yaml",
            content: format!(
                r#"name: {name}
role: tool
repo_type: docs
standards_version: "{version}"
owners:
  - "@architecture-team"

contracts:
  implements: []
  depends_on: []

crate_classes: []

exceptions: []
"#
            ),
        },
        FileToCreate {
            path: "README.md",
            content: format!(
                r#"# {name}

Source of truth for the Hologram ecosystem's architecture decisions, policies, and standards.

This repository contains:
- **Architecture Decision Records** (`specs/adrs/`) — the *why* behind every rule
- **Machine-readable policies** (`policies/`) — rules that `archon verify` enforces
- **Sync templates** (`templates/`) — content pushed to downstream repos via `archon sync`
- **Contract definitions** (`specs/contracts/`) — interfaces between repos
- **Ecosystem registry** (`ecosystem/`) — all known repos, their roles, and relationships
- **Standards documentation** (`standards/`) — human-readable standards and migration guides

## How it works

```
{name}               archon                   downstream repos
(this repo)               (enforcement tool)           (hologram, hologram-ai, ...)
─────────────────    ─────────────────────       ─────────────────────────────
specs/adrs/*     ──▶ AI context for doc gen
policies/*.yaml  ──▶ verify evaluates rules  ──▶ CI blocks non-conformant PRs
templates/*      ──▶ sync pushes content     ──▶ AGENTS.md, CLAUDE.md updated
ecosystem/*      ──▶ graph builds eco map    ──▶ dashboard, drift detection
```

**archon never writes to this repo.** The flow is strictly one-way: architecture → tool → downstream repos.

## Quick start

```bash
# Add a new architecture decision
archon adr new --title "Require API documentation"

# Test a policy change against all repos
for repo in ../hologram ../hologram-ai; do
  (cd "$repo" && archon verify --arch-root ../{name})
done

# Sync updated templates to downstream repos
for repo in ../hologram ../hologram-ai; do
  (cd "$repo" && archon sync --arch-root ../{name})
done
```

## Standards version: {version}
"#
            ),
        },
        // ── ADRs ────────────────────────────────────────────────────
        FileToCreate {
            path: "specs/adrs/0001-repo-boundaries.md",
            content: format!(
                r#"# ADR-0001: Repository Boundaries

## Status
Proposed

## Context
The Hologram ecosystem spans multiple repositories. We need clear boundaries defining which repos exist, what each repo is responsible for, and how they relate to each other.

## Decision
We define the following repository roles:
- **core**: foundational runtime and compiler repos (hologram, hologram-ai)
- **extension**: repos that extend core capabilities (hologram-sandbox)
- **library**: shared libraries and SDKs (hologram-sdk)
- **service**: deployed services (hologram-website)
- **tool**: development and governance tools (archon)

Each repo declares its role in `hologram.repo.yaml`. The role determines which policies apply and what contracts are expected.

## Consequences
- Every repo in the ecosystem must have a `hologram.repo.yaml` with a valid role
- `archon verify` checks that role declarations match actual structure
- New repos must be registered in `ecosystem/repos.yaml`
- The role taxonomy can be extended via new ADRs

## Standards Version
{version}
"#
            ),
        },
        FileToCreate {
            path: "specs/adrs/0002-contract-model.md",
            content: format!(
                r#"# ADR-0002: Contract Model

## Status
Proposed

## Context
Repos in the ecosystem depend on each other through shared interfaces (crates, APIs, file formats). These dependencies need to be declared and tracked to prevent breaking changes and ensure compatibility.

## Decision
We adopt a **contract model** where:
- A contract is a named interface (e.g., `hologram-execution-plan`)
- Repos declare which contracts they `implement` (provide) and `depend_on` (consume)
- Contract definitions live in `specs/contracts/` in this architecture repo
- `archon graph` visualizes the contract dependency graph

Contract declarations live in each repo's `hologram.repo.yaml`:
```yaml
contracts:
  implements:
    - sandbox-runtime
  depends_on:
    - hologram-execution-plan
```

## Consequences
- Breaking a contract requires an ADR and coordination with all consumers
- `archon graph` can detect circular dependencies and orphaned contracts
- New contracts must be defined in `specs/contracts/` before repos can declare them

## Standards Version
{version}
"#
            ),
        },
        FileToCreate {
            path: "specs/adrs/0003-naming-conventions.md",
            content: format!(
                r#"# ADR-0003: Naming Conventions

## Status
Proposed

## Context
With multiple repos and many crates, consistent naming prevents confusion and makes the ecosystem discoverable.

## Decision
- All ecosystem crate names use the `hologram-` prefix (never `holo-`)
- Repository names use `hologram-` prefix for ecosystem repos
- ADR files use `NNNN-kebab-case-title.md` format
- Policy rule IDs use category prefixes: `STR-` (structural), `POL-` (policy/governance), `ARCH-` (architectural)

## Consequences
- `archon verify` can validate crate naming in `Cargo.toml`
- The naming convention is documented in AGENTS.md managed sections so AI agents follow it
- Exceptions to naming rules require an explicit exception in `hologram.repo.yaml`

## Standards Version
{version}
"#
            ),
        },
        FileToCreate {
            path: "specs/adrs/0004-standards-versioning.md",
            content: format!(
                r#"# ADR-0004: Standards Versioning

## Status
Proposed

## Context
As the ecosystem evolves, policies and requirements change. Repos need a way to declare which version of standards they follow, and the ecosystem needs to track drift.

## Decision
- Standards use **date-based versions**: `YYYY.MM` format (e.g., `{version}`)
- Each repo declares its `standards_version` in `hologram.repo.yaml`
- Policy files are versioned alongside standards
- Bumping the standards version requires a migration guide in `standards/migration/`
- `archon verify` warns when a repo is behind the current standards version

## Consequences
- Standards versions are monotonically increasing
- Repos can upgrade at their own pace (with drift visibility)
- Breaking changes (new error-level rules) require a version bump
- `archon status` shows each repo's version relative to current

## Standards Version
{version}
"#
            ),
        },
        FileToCreate {
            path: "specs/adrs/0005-sync-model.md",
            content: format!(
                r#"# ADR-0005: File Sync Model

## Status
Proposed

## Context
Architecture decisions need to reach AI agents working in downstream repos. Manually copying rules is error-prone and doesn't scale.

## Decision
We define three ownership levels for files synced from this architecture repo to downstream repos:

- **fully-managed**: archon owns the entire file. Overwritten from source on every sync.
- **section-managed**: archon owns content between `<!-- ARCHON:MANAGED:BEGIN -->` and `<!-- ARCHON:MANAGED:END -->` markers. Handwritten content outside markers is preserved.
- **unmanaged**: archon never modifies the file.

Each downstream repo declares ownership in `sync-manifest.yaml`:
```yaml
files:
  - path: AGENTS.md
    ownership: section-managed
    source: templates/agents-managed-section.md
```

## Consequences
- `archon sync` only updates files that have changed (SHA-256 hash tracking)
- AI merge is used when both upstream and local versions have diverged
- Downstream repos control which files are managed via their sync manifest
- Templates in this repo are the single source of truth for managed content

## Standards Version
{version}
"#
            ),
        },
        FileToCreate {
            path: "specs/adrs/0006-agent-guidance.md",
            content: format!(
                r#"# ADR-0006: Agent Guidance Model

## Status
Proposed

## Context
AI agents (Claude, Codex, etc.) build most of the ecosystem's code. They need consistent, up-to-date guidance about ecosystem conventions, rules, and architecture.

## Decision
- Every repo must have an `AGENTS.md` file
- `AGENTS.md` contains a managed section (synced from this repo) with ecosystem-wide rules
- `AGENTS.md` also contains project-specific guidance written by the repo's maintainers
- `CLAUDE.md` follows the same pattern for Claude-specific context
- `hologram.repo.yaml` provides machine-readable context about the repo's role and contracts

## Consequences
- AI agents read `AGENTS.md` in their working repo — they don't need to access this architecture repo
- `archon sync` keeps the managed sections current
- `archon verify` checks that `AGENTS.md` exists (STR-002)
- New ecosystem-wide guidance is added to `templates/agents-managed-section.md` in this repo

## Standards Version
{version}
"#
            ),
        },
        // ── Policies ────────────────────────────────────────────────
        FileToCreate {
            path: "policies/structural.yaml",
            content: format!(
                r#"version: "{version}"
rules:
  - id: STR-001
    category: structural
    severity: error
    description: "hologram.repo.yaml must exist"
    check:
      type: file_exists
      path: "hologram.repo.yaml"

  - id: STR-002
    category: structural
    severity: error
    description: "AGENTS.md must exist"
    check:
      type: file_exists
      path: "AGENTS.md"

  - id: STR-003
    category: structural
    severity: warning
    description: "specs/docs/ directory should exist"
    check:
      type: dir_exists
      path: "specs/docs"

  - id: STR-004
    category: structural
    severity: warning
    description: "specs/docs/architecture.md should exist"
    check:
      type: file_exists
      path: "specs/docs/architecture.md"
"#
            ),
        },
        FileToCreate {
            path: "policies/governance.yaml",
            content: format!(
                r#"version: "{version}"
rules:
  - id: POL-001
    category: policy
    severity: error
    description: "standards_version must be present and valid"
    check:
      type: metadata_field
      field: standards_version
      condition: present

  - id: POL-002
    category: policy
    severity: warning
    description: "owners must not be empty"
    check:
      type: metadata_field
      field: owners
      condition: non_empty

  - id: POL-003
    category: policy
    severity: warning
    description: "standards version should be current"
    check:
      type: standards_version
      minimum: "{version}"
"#
            ),
        },
        FileToCreate {
            path: "policies/architectural.yaml",
            content: format!(
                r#"version: "{version}"
rules:
  - id: ARCH-001
    category: architectural
    severity: warning
    description: "public-api crates must not depend on binary crates"
    check:
      type: dependency_direction
      disallowed:
        - from: public-api
          to: binary

  - id: ARCH-002
    category: architectural
    severity: warning
    description: "all workspace crates should have a declared class"
    check:
      type: crate_taxonomy
      require_classes: true
"#
            ),
        },
        // ── Templates ───────────────────────────────────────────────
        FileToCreate {
            path: "templates/agents-managed-section.md",
            content: r#"## Ecosystem Rules

These rules apply to all repositories in the Hologram ecosystem.

### Naming
- Use the `hologram-` prefix for all crate names (never `holo-`)
- Follow kebab-case for crate and repo names

### Code Quality
- Run `cargo clippy -- -D warnings` before committing Rust changes
- All public APIs must have documentation comments
- No `unwrap()` in library code — use proper error handling

### Architecture
- Follow ADR decisions from `hologram-architecture`
- Declare contracts in `hologram.repo.yaml`
- Do not introduce cross-repo dependencies without an ADR

### Documentation
- Keep `specs/docs/architecture.md` up to date with structural changes
- Update `AGENTS.md` when adding new conventions or rules
"#
            .to_string(),
        },
        FileToCreate {
            path: "templates/claude-managed-section.md",
            content: r#"## Ecosystem Context

This repository is part of the **Hologram** ecosystem — a multi-repo project governed by shared architecture decisions.

### Key files
- `hologram.repo.yaml` — this repo's role, contracts, and standards version
- `AGENTS.md` — guidance for AI agents (includes ecosystem-wide rules)
- `specs/docs/architecture.md` — project-specific architecture documentation

### Standards
- Standards version is declared in `hologram.repo.yaml`
- Run `archon verify` to check conformance
- Run `archon sync` to pull latest managed content from the architecture repo

### Conventions
- Use `hologram-` prefix for crate names
- Follow ADR decisions (see `hologram-architecture/specs/adrs/`)
- Declare inter-repo dependencies as contracts in `hologram.repo.yaml`
"#
            .to_string(),
        },
        FileToCreate {
            path: "templates/upstream-architecture.md",
            content: r#"# Hologram Ecosystem Architecture

This document is managed by `archon sync` from the architecture repository. Do not edit it directly in downstream repos.

## Ecosystem Overview

The Hologram ecosystem is a collection of repositories that work together to provide a modular execution runtime with AI-driven compilation and sandboxed execution.

## Repository Roles

| Role | Description | Examples |
|---|---|---|
| core | Foundational runtime and compiler | hologram, hologram-ai |
| extension | Extends core capabilities | hologram-sandbox |
| library | Shared libraries and SDKs | hologram-sdk |
| service | Deployed services | hologram-website |
| tool | Development and governance | archon |

## Contracts

Repos interact through **contracts** — named interfaces declared in `hologram.repo.yaml`. A contract defines what a repo provides (`implements`) and what it consumes (`depends_on`).

## Standards

The ecosystem uses date-based standards versioning (`YYYY.MM`). Each repo declares which standards version it follows. `archon verify` checks conformance.

## Governance

Architecture decisions are recorded as ADRs in the architecture repository. Policy rules encode those decisions as machine-readable checks. `archon` enforces them across all repos.
"#
            .to_string(),
        },
        // ── Standards ───────────────────────────────────────────────
        FileToCreate {
            path: "standards/current.md",
            content: format!(
                r#"# Standards Version {version}

## Required files
- `hologram.repo.yaml` — repo identity (STR-001)
- `AGENTS.md` — agent guidance (STR-002)

## Recommended files
- `specs/docs/` directory (STR-003)
- `specs/docs/architecture.md` (STR-004)

## Metadata requirements
- `standards_version` must be present (POL-001)
- `owners` should not be empty (POL-002)
- `standards_version` should be `{version}` or later (POL-003)

## Architectural rules
- Public-API crates must not depend on binary crates (ARCH-001)
- All workspace crates should have a declared class (ARCH-002)
"#
            ),
        },
        FileToCreate {
            path: "standards/changelog.md",
            content: format!(
                r#"# Standards Changelog

## {version} (initial)

- Initial standards version
- Structural rules: STR-001 through STR-004
- Governance rules: POL-001 through POL-003
- Architectural rules: ARCH-001, ARCH-002
"#
            ),
        },
        // ── Ecosystem ───────────────────────────────────────────────
        FileToCreate {
            path: "ecosystem/repos.yaml",
            content: format!(
                r#"# Hologram Ecosystem Registry
# This is the authoritative list of all repos in the ecosystem.
# archon graph reads this to build the contract dependency map.

repos: []

# Example entry:
#  - name: hologram
#    role: core
#    standards_version: "{version}"
#    contracts:
#      implements: [hologram-execution-plan, hologram-graph-model]
#      depends_on: []
#    url: https://github.com/org/hologram
"#
            ),
        },
        FileToCreate {
            path: "ecosystem/contract-graph.yaml",
            content: r#"# Contract Graph
# This file is generated by archon graph and should not be edited manually.
# Run: archon graph --repos-dir .. --format yaml > ecosystem/contract-graph.yaml

contracts: []
"#
            .to_string(),
        },
        // ── Contracts placeholder ───────────────────────────────────
        FileToCreate {
            path: "specs/contracts/.gitkeep",
            content: String::new(),
        },
        // ── Plans placeholder ───────────────────────────────────────
        FileToCreate {
            path: "specs/plans/.gitkeep",
            content: String::new(),
        },
        // ── AGENTS.md ───────────────────────────────────────────────
        FileToCreate {
            path: "AGENTS.md",
            content: format!(
                r#"# AGENTS.md

This document provides guidance for automated agents operating in **`{name}`**.

---

## Repository Purpose

`{name}` is the **source of truth** for the Hologram ecosystem's architecture. It contains decisions, policies, templates, and standards — not code.

Standards version: `{version}`

---

## What This Repo Contains

| Directory | Purpose |
|---|---|
| `specs/adrs/` | Architecture Decision Records — the *why* behind every rule |
| `policies/` | Machine-readable rules enforced by `archon verify` |
| `templates/` | Content synced to downstream repos via `archon sync` |
| `standards/` | Human-readable standards documentation |
| `ecosystem/` | Registry of all repos and their relationships |
| `specs/contracts/` | Contract definitions between repos |

## Rules for Agents

1. **ADRs are append-only** — never delete or significantly alter an accepted ADR. Supersede it with a new one instead.
2. **Policy rules must trace to an ADR** — every rule in `policies/` should reference the ADR that motivated it.
3. **Templates affect all downstream repos** — changes to `templates/` propagate on `archon sync`. Be deliberate.
4. **Use sequential numbering for ADRs** — check `specs/adrs/` for the next available number.
5. **Bump standards version for breaking changes** — new error-level rules require a version bump and migration guide.

---

<!-- ARCHON:MANAGED:BEGIN -->
This section is managed by archon. Do not edit manually.
<!-- ARCHON:MANAGED:END -->
"#
            ),
        },
        // ── CLAUDE.md ───────────────────────────────────────────────
        FileToCreate {
            path: "CLAUDE.md",
            content: format!(
                r#"# CLAUDE.md

Context for Claude Code sessions in **`{name}`**.

## What this repo is

This is the architecture repository for the Hologram ecosystem. It does NOT contain code — only decisions, policies, templates, and standards.

The enforcement tool is `archon` (a separate repo).

## Key directories

- `specs/adrs/` — Architecture Decision Records
- `policies/` — YAML rules loaded by `archon verify`
- `templates/` — Content synced to downstream repos
- `ecosystem/repos.yaml` — Registry of all ecosystem repos

## Standards version: {version}

## Workflow for changes

1. Create an ADR explaining the decision
2. Encode the decision as a rule in `policies/`
3. Update `templates/` if agents need to see the new guidance
4. Bump standards version if it's a breaking change

---

<!-- ARCHON:MANAGED:BEGIN -->
This section is managed by archon. Do not edit manually.
<!-- ARCHON:MANAGED:END -->
"#
            ),
        },
        // ── .gitignore ──────────────────────────────────────────────
        FileToCreate {
            path: ".gitignore",
            content: r#".archon/
.claude/
"#
            .to_string(),
        },
    ]
}
