# arch-tool

Architecture governance tool for the Hologram ecosystem.

arch-tool is the **enforcement engine** that keeps multiple repositories aligned with shared architecture decisions. It reads decisions from the architecture repository (`hologram-architecture`), encodes them as machine-readable policies, and pushes them into downstream repos via file sync, conformance checking, and AI-guided documentation.

**Two repositories, distinct roles:**

| Repository | Role |
|---|---|
| `hologram-architecture` | **Source of truth** — architecture decisions (ADRs), policies, standards, templates, contract definitions |
| `arch-tool` (this repo) | **Enforcement tool** — reads those decisions and applies them across the ecosystem |

Architecture decisions are made in `hologram-architecture`. This repo builds the tool that enforces them.

---

## Why This Exists

The Hologram ecosystem is built across multiple repositories:

| Repository | Role | What it does |
|---|---|---|
| `hologram` | core | Execution runtime, graph representation, memory contracts |
| `hologram-ai` | core | AI compiler, model import, `.holo` archive generation |
| `hologram-sandbox` | extension | Process/WASM/microVM isolation runtimes |
| `hologram-sdk` | library | Developer SDK |
| `hologram-website` | service | Documentation site |
| `hologram-architecture` | authority | Architecture decisions, policies, standards |
| **`arch-tool`** | **tool** | **This repo. Enforcement CLI + server.** |

These repos are mostly built by AI agents (Claude, Codex, etc.). Each agent works in its own repo, making local decisions. Without enforcement, the repos drift apart — incompatible interfaces, missing documentation, broken contracts, inconsistent conventions.

arch-tool bridges the gap between high-level architecture decisions and implementation-level code:

- **Reads** architecture decisions from `hologram-architecture` (ADRs, policies, templates)
- **Generates** the right documentation and metadata when bootstrapping a new repo
- **Syncs** managed files and sections when architecture evolves
- **Enforces** standards through automated conformance checking (locally and in CI)
- **Informs** AI agents working in downstream repos about the ecosystem's structure

---

## How Decisions Stay in Sync

This is the core question: architecture decisions live in `hologram-architecture`, but AI agents work in implementation repos. How do decisions reach the agents?

### The sync pipeline

```
hologram-architecture/          arch-tool (this tool)           downstream repos
┌──────────────────────┐        ┌──────────────────┐          ┌──────────────────┐
│ specs/adrs/           │        │                  │          │                  │
│   0001-boundaries.md  │───────▶│  reads ADRs &    │          │                  │
│   0002-contracts.md   │        │  policies from   │  sync    │  AGENTS.md       │
│                       │        │  arch repo       │─────────▶│  (managed        │
│ policies/             │        │                  │          │   sections)      │
│   structural.yaml     │───────▶│  evaluates rules │          │                  │
│   governance.yaml     │        │  against repo    │  verify  │  hologram.repo   │
│   architectural.yaml  │        │  state           │─────────▶│  .yaml           │
│                       │        │                  │          │                  │
│ templates/            │        │  generates docs  │  init    │  specs/docs/     │
│   agents-section.md   │───────▶│  with AI context │─────────▶│  architecture.md │
│   upstream-arch.md    │        │                  │          │  development.md  │
└──────────────────────┘        └──────────────────┘          └──────────────────┘
```

### Step by step

1. **Decision is made** in `hologram-architecture`: a new ADR is written (e.g., "all repos must declare crate classes"), and the rule is encoded in `policies/architectural.yaml`

2. **arch-tool reads it**: when you run `arch-tool sync` or `arch-tool verify`, arch-tool locates the architecture repo (via `--arch-root`, `ARCH_TOOL_ROOT` env var, or sibling directory auto-discovery) and loads the current policies and templates

3. **Sync pushes to downstream repos**: `arch-tool sync` reads the downstream repo's `sync-manifest.yaml` and updates managed files:
   - **Fully-managed files** are overwritten from the architecture repo's templates
   - **Section-managed files** (like AGENTS.md) have their `<!-- ARCH-TOOL:MANAGED:BEGIN/END -->` sections replaced while preserving project-specific content outside those markers
   - **Unmanaged files** are left untouched

4. **AI agents read the synced files**: when an AI agent starts working in hologram-sandbox, it reads the updated AGENTS.md and CLAUDE.md — which now contain the latest ecosystem rules synced from the architecture repo

5. **Verify enforces in CI**: `arch-tool verify` runs as a CI check on every PR, catching conformance issues before they merge

### What this means in practice

- **You never manually copy rules between repos.** `arch-tool sync` does it.
- **AI agents don't need to read the architecture repo.** They read AGENTS.md in their own repo, which arch-tool keeps current.
- **New rules automatically propagate.** Add a rule to `hologram-architecture/policies/`, run `arch-tool sync` across repos (or have CI do it), and every agent sees the new guidance.
- **Drift is visible.** `arch-tool verify` and `arch-tool status` show exactly where each repo stands relative to current standards.

### How architecture decisions are made

This happens in `hologram-architecture`, not here. The workflow:

1. **Prompt**: Write a prompt in `hologram-architecture/specs/prompts/` describing the architectural question
2. **Research**: AI or human produces analysis — stored in `hologram-architecture/specs/plans/`
3. **Decision**: Record it as an ADR in `hologram-architecture/specs/adrs/`
4. **Encode**: Translate the decision into a machine-readable rule in `hologram-architecture/policies/`
5. **Sync**: Run `arch-tool sync` in downstream repos to push the new guidance
6. **Enforce**: `arch-tool verify` checks it automatically from this point forward

Example flow:
- You decide "all repos must have an AGENTS.md file" → write ADR in hologram-architecture
- Encode as policy rule `STR-002` in `hologram-architecture/policies/structural.yaml`
- `arch-tool sync` updates the managed section of AGENTS.md in every downstream repo to include the new rule context
- `arch-tool verify` checks for AGENTS.md existence in every repo
- AI agents reading AGENTS.md see the ecosystem-wide guidance

---

## The Configuration Files

### `hologram.repo.yaml` — Repository Identity Card

Every downstream repository gets this file. It declares what the repo IS in the ecosystem:

```yaml
name: hologram-sandbox
role: extension                 # core | extension | tool | service | library
repo_type: rust-workspace       # rust-workspace | rust-binary | mixed
standards_version: "2026.03"    # date-based: which standards version this repo follows
architecture_version: "1.0"     # which architecture version this repo targets
owners:
  - "@core-team"

contracts:
  implements:                   # what this repo PROVIDES to the ecosystem
    - sandbox-runtime           # other repos can depend on this contract
    - process-isolation
  depends_on:                   # what this repo CONSUMES from other repos
    - hologram-execution-plan   # provided by the hologram repo

crate_classes:                  # classify each crate in a Rust workspace
  - name: hologram-sandbox
    class: public-api           # public-api | internal | binary | test-support
  - name: hologram-sandbox-wasm
    class: internal
  - name: hologram-sandbox-process
    class: internal

exceptions:                     # approved deviations from policy rules
  - id: EXC-2026-001
    rule: STR-004               # which rule is excepted
    expires: "2026-06-01"       # must expire — no permanent exceptions
```

#### How it's created

```bash
cd ~/work/uor/hologram/hologram-sandbox
arch-tool init
```

arch-tool does the following:
1. Reads the repo's `Cargo.toml` to understand what kind of project it is
2. If AI is available (ANTHROPIC_API_KEY or `claude` CLI), asks AI to determine the appropriate role, contracts, and relevant documentation
3. If no AI, uses the `--profile` flag to select a predefined configuration
4. Generates `hologram.repo.yaml` with the right declarations
5. Creates AGENTS.md, CLAUDE.md, specs/docs/architecture.md, specs/docs/development.md
6. AI fills `<!-- TODO -->` placeholders with project-specific content derived from the repo's actual code and the ecosystem's ADRs

#### How it's updated

Edit it directly. It's YAML — humans and AI agents both read and write it.

When the ecosystem adds a new contract, you add it to `contracts.implements` or `contracts.depends_on`. When you add a new crate to the workspace, add it to `crate_classes`. When you need to break a rule temporarily, add an exception with an expiry date.

`arch-tool verify` will tell you if your declarations are inconsistent (e.g., you claim to implement a contract but the required files don't exist).

#### What it enables

- **`arch-tool verify`**: checks that the repo actually conforms to what it declares
- **`arch-tool graph`**: builds the ecosystem's contract map — who provides what, who depends on what
- **`arch-tool status`**: shows where the repo stands relative to current standards
- **AI agents**: read this file to understand the repo's role and responsibilities

---

### Policy Files — The Rules

Stored in this repo at `policies/*.yaml`. These are the machine-readable rules that `arch-tool verify` enforces:

```yaml
# policies/structural.yaml
version: "2026.03"
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
```

```yaml
# policies/governance.yaml
version: "2026.03"
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
      minimum: "2026.03"
```

```yaml
# policies/architectural.yaml
version: "2026.03"
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
```

#### How rules are added

1. Make an architecture decision (ADR) about what should be required
2. Add the rule to the appropriate policy file
3. Bump the standards version if it's a breaking change
4. Run `arch-tool verify` across the ecosystem to see the impact
5. Downstream repos either comply or file an exception

arch-tool ships with built-in defaults for these rules, so it works even without the policy files present. The policy files override and extend the defaults.

#### Rule categories

- **structural**: "Does this file exist?" — Required files, directories, configs
- **policy**: "Is the metadata correct?" — Valid standards version, owners declared, required fields present
- **architectural**: "Are the code-level constraints respected?" — Dependency direction, crate taxonomy, contract consistency

#### Severity levels

- **error**: Must pass. `arch-tool verify` exits 1 if any error-level rule fails. Use in CI to block merges.
- **warning**: Should pass. Reported but doesn't fail the build. Use `--strict` to promote warnings to errors.
- **info**: Advisory. Information for awareness, never fails.

---

### `sync-manifest.yaml` — File Ownership Model

When architecture evolves, downstream repos need to be updated. This file controls which files arch-tool manages:

```yaml
version: "2026.03"
files:
  # Holoarch fully owns this file — overwritten from source on every sync
  - path: specs/docs/upstream-architecture.md
    ownership: fully-managed
    source: templates/upstream-architecture.md

  # Holoarch owns the marked sections; everything else is handwritten
  - path: AGENTS.md
    ownership: section-managed
    source: templates/agents-managed-section.md

  # Holoarch owns the marked sections; everything else is handwritten
  - path: CLAUDE.md
    ownership: section-managed
    source: templates/claude-managed-section.md

  # Holoarch never touches this — it's project-specific
  - path: specs/docs/architecture.md
    ownership: unmanaged
```

#### The three ownership levels

**fully-managed**: arch-tool owns the entire file. On `arch-tool sync`, the file is overwritten from the source template in this repo. Use for: shared architecture summaries, upstream standards docs, generated configs.

**section-managed**: arch-tool owns the content between `<!-- ARCH-TOOL:MANAGED:BEGIN -->` and `<!-- ARCH-TOOL:MANAGED:END -->` markers. Everything outside those markers is handwritten and preserved. On `arch-tool sync`, only the managed sections are replaced. Use for: AGENTS.md (shared ecosystem rules + project-specific rules), CLAUDE.md (shared context + project-specific context).

Example of a section-managed file:
```markdown
# AGENTS.md

This document provides guidance for agents in **hologram-sandbox**.

## Project-Specific Rules
(this part is written by the project team and never overwritten)

<!-- ARCH-TOOL:MANAGED:BEGIN -->
## Ecosystem Rules
(this part is managed by arch-tool and updated on sync)
- Use the hologram- prefix for all crate names
- Follow ADR decisions from arch-tool
- Run cargo clippy -- -D warnings before committing
<!-- ARCH-TOOL:MANAGED:END -->
```

**unmanaged**: arch-tool never modifies this file. It's entirely project-owned. Use for: project-specific architecture docs, READMEs, implementation details.

---

## Commands

### `arch-tool init` — Bootstrap a Repository

Creates `hologram.repo.yaml` and base documentation in a downstream repo.

```bash
cd ~/work/uor/hologram/hologram-new-project
arch-tool init                              # AI selects relevant docs
arch-tool init --profile runtime-system     # use a predefined profile
arch-tool init --standards-version 2026.03  # set specific version
arch-tool init --dry-run                    # preview without writing
arch-tool init --force                      # overwrite existing files
```

**What it creates:**
| File | Purpose |
|---|---|
| `hologram.repo.yaml` | Repo identity — role, contracts, standards version |
| `AGENTS.md` | Guidance for AI agents working in this repo |
| `CLAUDE.md` | Context for Claude Code sessions |
| `specs/docs/architecture.md` | Project-specific architecture documentation |
| `specs/docs/development.md` | Development guide |
| *(AI-selected optional docs)* | runtime.md, security.md, api.md, etc. |

**AI behavior during init:**
1. Reads the repo's Cargo.toml and existing files
2. Asks AI: "which optional docs are relevant to this project?"
3. AI responds with a list (e.g., `["specs/docs/runtime.md", "specs/docs/security.md"]`)
4. Generates those additional docs
5. Fills `<!-- TODO -->` placeholders with project-specific content derived from the repo's code and the ecosystem's ADRs

If no AI backend is available, falls back to template-based generation with TODO placeholders.

### `arch-tool verify` — Check Conformance

Runs all policy rules against a repository and reports results.

```bash
arch-tool verify                                    # colored terminal report
arch-tool verify --format json                      # machine-readable for CI
arch-tool verify --strict                           # warnings become errors
arch-tool verify --arch-root ~/work/uor/hologram/holoarch  # explicit path to this repo
```

**How it works:**
1. Finds `hologram.repo.yaml` in the current directory (walks up parent directories)
2. Loads policy rules from this repo's `policies/` directory (falls back to built-in defaults)
3. Loads exceptions from the repo's `hologram.repo.yaml`
4. Runs every rule:
   - **Structural**: checks that required files and directories exist
   - **Policy**: validates metadata fields, standards version, owner declarations
   - **Architectural**: checks dependency direction, crate taxonomy
5. Exceptions suppress failing rules (marked as "excepted" in the report)
6. Prints report; exits 1 if any error-level rules fail

**Example output:**
```
arch-tool verify: hologram-sandbox
  standards version: 2026.03

  ✓ [STR-001] structural: hologram.repo.yaml exists
  ✓ [STR-002] structural: AGENTS.md exists
  ⚠ [STR-003] structural: specs/docs/ not found
  ✓ [STR-004] structural: specs/docs/architecture.md exists [excepted]
  ✓ [POL-001] policy: standards_version: 2026.03
  ⚠ [POL-002] policy: owners list is empty

  PASS 4/6 checks passed, 2 warnings
```

**In CI:**
```yaml
# .github/workflows/verify.yml
- name: Architecture conformance
  run: arch-tool verify --strict --format json
```

### `arch-tool status` — Read-Only Summary

Same as verify but never exits with a non-zero code. For informational use.

```bash
arch-tool status
arch-tool status --format json
```

### `arch-tool sync` — Sync Managed Files *(Phase 2)*

Pull updated managed files from this repo into a downstream repo.

```bash
arch-tool sync --dry-run                                    # preview changes
arch-tool sync --arch-root ~/work/uor/hologram/holoarch     # explicit path
arch-tool sync --force                                      # overwrite local changes
```

**What it does:**
1. Reads `sync-manifest.yaml` from the downstream repo
2. For **fully-managed** files: compares SHA-256 hash with source; overwrites if changed
3. For **section-managed** files: extracts managed sections from source; replaces only those sections in the downstream file
4. For **unmanaged** files: skips entirely
5. Records hashes in `.arch-tool/sync-state.yaml` for change detection
6. AI merges when both versions have changed (preserves handwritten content)

### `arch-tool adr new` — Create Architecture Decision *(Phase 2)*

Create a new ADR in this repo:

```bash
arch-tool adr new --title "Use YAML for all config files"
arch-tool adr list
```

Creates `specs/adrs/NNNN-use-yaml-for-all-config-files.md` with the next sequential number.

### `arch-tool exception new` — Declare Policy Deviation *(Phase 2)*

Declare an approved exception in a downstream repo's `hologram.repo.yaml`:

```bash
arch-tool exception new --rule STR-003 --reason "Legacy layout, migrating Q2" --expires 2026-06-01
arch-tool exception list
```

Adds to the repo's `hologram.repo.yaml`:
```yaml
exceptions:
  - id: EXC-2026-001
    rule: STR-003
    expires: "2026-06-01"
```

### `arch-tool graph` — Ecosystem Map *(Phase 3)*

Build and display the contract dependency graph across all repos:

```bash
arch-tool graph --repos-dir ~/work/uor/hologram   # scan all sibling repos
arch-tool graph --format dot                       # graphviz output
arch-tool graph --format json                      # machine-readable
```

**What it shows:**
```
hologram
  implements: hologram-execution-plan, hologram-graph-model
  depends_on: (none)

hologram-ai
  implements: hologram-ai-compiler
  depends_on: hologram-execution-plan

hologram-sandbox
  implements: sandbox-runtime, process-isolation
  depends_on: hologram-execution-plan

CONTRACT GRAPH:
  hologram-execution-plan: hologram → hologram-ai, hologram-sandbox
  hologram-graph-model: hologram → (no consumers)
  sandbox-runtime: hologram-sandbox → (no consumers)
```

### `arch-tool serve` — HTTP API *(Phase 3)*

Run a governance server that other tools and dashboards can query:

```bash
arch-tool serve --port 8080 --repos-dir ~/work/uor/hologram
```

| Endpoint | Method | Description |
|---|---|---|
| `/repos` | GET | List all repos with conformance status |
| `/repos/:name/status` | GET | Detailed conformance report |
| `/verify` | POST | Run conformance on a repo path |
| `/graph` | GET | Contract dependency graph (JSON or DOT) |
| `/policies` | GET | Active policy rules |
| `/drift` | GET | Repos behind on standards version |

---

## AI Integration

arch-tool uses AI at every stage:

| Stage | How AI helps |
|---|---|
| **init** | Analyzes Cargo.toml and existing files to determine repo role, contracts, and relevant docs |
| **init** | Fills `<!-- TODO -->` placeholders with project-specific content using ADR context |
| **sync** | Merges conflicting doc versions — preserves handwritten content while adding new guidance |
| **doc selection** | Selects which optional docs are relevant when syncing |

**Backend detection** (automatic, in priority order):
1. `ANTHROPIC_API_KEY` environment variable → calls Anthropic Messages API directly
2. `claude` binary in PATH → pipes prompt to `claude --print`

AI is always optional. If no backend is available, arch-tool falls back to templates with TODO placeholders. The workflow is: generate templates → human or AI fills them in later.

---

## Intended Workflow for AI Agents

This is designed for a workflow where AI agents build the ecosystem's repos. Here's how it works:

### When an AI agent starts working in a downstream repo

1. The agent reads `AGENTS.md` — which contains ecosystem-wide rules synced from `hologram-architecture`, plus project-specific guidance
2. The agent reads `CLAUDE.md` — which has project context and architecture pointers
3. The agent reads `hologram.repo.yaml` — which tells it the repo's role, contracts, and standards version
4. The agent reads `specs/docs/architecture.md` — which has the project-specific architecture

These files were generated by `arch-tool init` and kept current by `arch-tool sync`. They contain the decisions made in `hologram-architecture`, translated into guidance the agent can follow. **The agent never needs to read `hologram-architecture` directly.**

### When you make an architecture decision

1. Write an ADR in `hologram-architecture/specs/adrs/`: "hologram-sandbox must not import directly from hologram-ai"
2. Encode the rule in `hologram-architecture/policies/architectural.yaml`
3. Update templates in `hologram-architecture/templates/` if the rule needs to appear in AGENTS.md managed sections
4. Run `arch-tool sync` in hologram-sandbox — the updated managed sections and policies flow into the repo
5. Run `arch-tool verify` to confirm the rule passes (or file an exception)
6. Next time an AI agent works in hologram-sandbox, it reads the updated AGENTS.md and follows the new rule

### When you bootstrap a new repo

```bash
mkdir hologram-new-thing && cd hologram-new-thing
cargo init --lib
arch-tool init --arch-root ~/work/uor/hologram/hologram-architecture
```

arch-tool reads the architecture repo's policies and templates, analyzes the new project, generates all governance files, fills in documentation with AI, and the repo is immediately conformant. An AI agent can start working in it and will have full context about the ecosystem.

### The information flow

```
hologram-architecture          arch-tool sync/init            AI agent in downstream repo
────────────────────          ──────────────────            ──────────────────────────
ADR: "no cross-imports
  between sandbox/ai"   ──▶   Encodes as ARCH-003    ──▶   Agent reads AGENTS.md:
                                                            "Do not import directly
Policy: ARCH-003 in                                          from hologram-ai"
  architectural.yaml    ──▶   Evaluates in verify    ──▶
                                                            Agent reads hologram.repo.yaml:
Template: updated                                           role=extension,
  agents-section.md     ──▶   Syncs managed section  ──▶   contracts.depends_on excludes
                              into AGENTS.md                hologram-ai
```

The architecture repo holds the truth. arch-tool is the conveyor belt. Downstream repos receive guidance in files that AI agents already know how to read.

---

## Standards Versioning

Standards use date-based versions: `2026.03`, `2026.06`, `2026.09`, etc.

- Each downstream repo declares its standards version in `hologram.repo.yaml`
- Policy rules are versioned alongside standards
- When the ecosystem advances (new rules, new requirements), bump the standards version
- `arch-tool verify` checks that repos aren't too far behind
- `arch-tool sync` helps repos catch up by pulling updated files
- `arch-tool graph --drift` shows which repos are behind

Drift is always explicit. A repo either conforms to its declared version, has approved exceptions, or is flagged as non-conformant. No silent drift.

---

## Repository Structure

### This repo (arch-tool) — the tool

```
arch-tool/
├── Cargo.toml                    Workspace root (10 crates)
├── Justfile                      Build/test/release recipes
├── README.md                     This file
│
├── specs/
│   ├── plans/                    Tool implementation plans
│   ├── prompts/                  Prompts for arch-tool development
│   ├── sprints/                  Archived sprints
│   └── SPRINT.md                 Current sprint
│
├── crates/
│   ├── arch-tool-core/            Core types: RepoMeta, StandardsVersion, paths
│   ├── arch-tool-policy/          Policy loading + evaluation engine
│   ├── arch-tool-verify/          Conformance checking
│   ├── arch-tool-sync/            File sync with managed sections
│   ├── arch-tool-adr/             ADR and exception management
│   ├── arch-tool-graph/           Contract/dependency graph
│   ├── arch-tool-templates/       Template rendering (embedded init files)
│   ├── arch-tool-ai/              AI backend (Anthropic API, Claude CLI)
│   ├── arch-tool-cli/             CLI binary (clap)
│   └── arch-tool-server/          HTTP server (axum)
│
└── tests/
    └── fixtures/                 Test repos for integration tests
```

### The architecture repo (hologram-architecture) — the authority

arch-tool reads from this repo at runtime. Its expected layout:

```
hologram-architecture/
├── specs/
│   └── adrs/                     Architecture Decision Records (ecosystem-wide)
│       ├── 0001-repo-boundaries.md
│       ├── 0002-contract-model.md
│       └── ...
│
├── policies/                     Machine-readable governance rules (YAML)
│   ├── structural.yaml           Required files and directories
│   ├── governance.yaml           Metadata and versioning rules
│   └── architectural.yaml        Dependency and taxonomy rules
│
└── templates/                    Templates synced to downstream repos
    ├── agents-managed-section.md   Managed section content for AGENTS.md
    ├── claude-managed-section.md   Managed section content for CLAUDE.md
    └── upstream-architecture.md    Shared architecture summary
```

arch-tool discovers the architecture repo automatically (sibling directory, `ARCH_TOOL_ROOT` env var, or `--arch-root` flag). If the architecture repo is not found, arch-tool falls back to built-in default policies.

---

## Creating an Architecture Repository

arch-tool reads decisions from a separate **architecture repository**. Here's how to create and manage one.

### Bootstrap

```bash
mkdir hologram-architecture && cd hologram-architecture
git init

# Create the directory structure
mkdir -p specs/adrs specs/plans specs/contracts policies templates standards ecosystem

# Self-govern: the architecture repo uses arch-tool too
arch-tool init --profile cli-tool
```

### Required structure

```
hologram-architecture/
├── specs/
│   ├── adrs/                        Architecture Decision Records
│   │   ├── 0001-repo-boundaries.md
│   │   ├── 0002-contract-model.md
│   │   └── ...
│   └── contracts/                   Contract definitions
│       ├── hologram-execution-plan.md
│       └── ...
│
├── policies/                        Machine-readable rules (YAML)
│   ├── structural.yaml              Required files/dirs (STR-xxx)
│   ├── governance.yaml              Metadata rules (POL-xxx)
│   └── architectural.yaml           Dependency rules (ARCH-xxx)
│
├── templates/                       Sync templates for downstream repos
│   ├── agents-managed-section.md    Content for AGENTS.md managed sections
│   ├── claude-managed-section.md    Content for CLAUDE.md managed sections
│   └── upstream-architecture.md     Shared architecture overview
│
├── standards/                       Human-readable standards docs
│   ├── current.md                   Current standards version
│   └── changelog.md                 Version history
│
└── ecosystem/                       Ecosystem registry
    ├── repos.yaml                   All repos with roles and contracts
    └── contract-graph.yaml          Contract relationships
```

### Add a policy rule

```bash
cd hologram-architecture

# 1. Write the ADR explaining why
arch-tool adr new --title "Require error handling strategy docs"

# 2. Add the machine-readable rule
cat >> policies/structural.yaml << 'EOF'
  - id: STR-005
    category: structural
    severity: warning
    description: "specs/docs/error-handling.md should exist"
    check:
      type: file_exists
      path: "specs/docs/error-handling.md"
EOF

# 3. Test impact across the ecosystem
for repo in ../hologram ../hologram-ai ../hologram-sandbox; do
  echo "==> $(basename $repo)"
  (cd "$repo" && arch-tool verify --arch-root ../hologram-architecture)
done

# 4. Commit
git add -A && git commit -m "feat(policy): require error handling docs (ADR-0007)"
```

### Sync decisions to downstream repos

After updating templates or policies in the architecture repo:

```bash
# Sync all repos
for repo in ../hologram ../hologram-ai ../hologram-sandbox; do
  (cd "$repo" && arch-tool sync --arch-root ../hologram-architecture)
done
```

Or set the `ARCH_TOOL_ROOT` env var once:

```bash
export ARCH_TOOL_ROOT=~/work/uor/hologram/hologram-architecture
cd ~/work/uor/hologram/hologram-sandbox
arch-tool sync       # auto-discovers the architecture repo
arch-tool verify     # checks against current policies
```

### Register a new repo

Add it to `ecosystem/repos.yaml`:

```yaml
repos:
  - name: hologram-new-thing
    role: extension
    standards_version: "2026.03"
    contracts:
      implements: []
      depends_on: [hologram-execution-plan]
    url: https://github.com/org/hologram-new-thing
```

Then bootstrap the repo:

```bash
cd ~/work/uor/hologram/hologram-new-thing
cargo init --lib
arch-tool init --arch-root ../hologram-architecture
```

### Bump standards version

```bash
cd hologram-architecture
# Update standards/current.md with new requirements
# Write migration guide in standards/migration/2026.03-to-2026.06.md
# Update policy files with new minimum version
git add -A && git commit -m "feat(standards): bump to 2026.06"

# Then update each downstream repo:
for repo in ../hologram ../hologram-ai ../hologram-sandbox; do
  (cd "$repo" && arch-tool sync && arch-tool verify)
done
```

---

## Development

```bash
# Install just: cargo install just

just ci             # Full CI: format check + clippy + tests
just test           # Run all workspace tests
just clippy         # Lint with warnings as errors
just build          # Debug build
just build-release  # Release build
just install        # Symlink release binary to ~/.local/bin/arch-tool

just run verify     # Run arch-tool with args via cargo
just init           # Shortcut for arch-tool init
just verify         # Shortcut for arch-tool verify
just status         # Shortcut for arch-tool status

just release 2.0.0  # Cut a release with changelog
just deps           # Show crate dependency tree
```

---

## Crate Dependency Graph

```
arch-tool-core             Foundation: types, paths, errors (no workspace deps)
  ↑
arch-tool-policy           Policy loading + evaluation
arch-tool-sync             File sync engine
arch-tool-adr              ADR + exception management
arch-tool-graph            Contract/dependency graph
arch-tool-templates        Template rendering
arch-tool-ai               AI backend integration
  ↑
arch-tool-verify           Conformance engine (core + policy)
  ↑
arch-tool-cli              CLI binary (depends on all library crates)
arch-tool-server           HTTP server (core + policy + verify + graph)
```

All logic lives in library crates. The CLI and server are thin consumers — no duplication.
