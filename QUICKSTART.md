# Quickstart

Get archon running in under 5 minutes.

## Install

```bash
# From source (requires Rust 1.70+)
git clone <repo-url> && cd archon
cargo install --path crates/archon-cli

# Or build without installing
cargo build --release
# Binary at: target/release/archon
```

Verify it works:

```bash
archon --version
archon --help
```

## 1. Set up AI authentication (optional but recommended)

archon uses AI for document generation, TODO filling, and intelligent merges. Without AI, it falls back to templates with `<!-- TODO -->` placeholders.

**Option A: OAuth login (uses your Claude or OpenAI subscription)**

```bash
archon auth login
# Select "claude" or "openai", then complete the browser OAuth flow

archon auth status
# Confirm credentials are stored
```

**Option B: API key**

```bash
export ANTHROPIC_API_KEY=sk-ant-...
```

**Option C: Claude CLI**

If you have [Claude Code](https://claude.ai/claude-code) installed, archon detects it automatically.

Backend priority: `ANTHROPIC_API_KEY` > `claude` CLI > stored OAuth credentials.

## 2. Bootstrap a downstream repo

Navigate to any Rust project and initialize it:

```bash
cd ~/work/my-project
archon init
```

This creates:

| File | Purpose |
|------|---------|
| `hologram.repo.yaml` | Repo identity — role, contracts, standards version |
| `AGENTS.md` | Guidance for AI agents working in this repo |
| `CLAUDE.md` | Context for Claude Code sessions |
| `specs/docs/architecture.md` | Project-specific architecture docs |
| `specs/docs/development.md` | Development guide |

With AI available, archon analyzes your `Cargo.toml` and fills in all `<!-- TODO -->` placeholders with project-specific content.

**Options:**

```bash
archon init --profile rust-workspace   # skip AI, use a preset profile
archon init --dry-run                  # preview without writing files
archon init --force                    # overwrite existing files
archon init --no-ai                    # skip AI even if available
```

## 3. Check conformance

```bash
archon verify
```

Reports which governance rules pass or fail:

```
archon verify: my-project
  standards version: 2026.03

  ✓ [STR-001] hologram.repo.yaml exists
  ✓ [STR-002] AGENTS.md exists
  ⚠ [STR-003] specs/docs/ not found
  ✓ [POL-001] standards_version: 2026.03

  PASS 3/4 checks passed, 1 warning
```

```bash
archon verify --strict     # treat warnings as errors (for CI)
archon verify --format json # machine-readable output
```

## 4. View status

```bash
archon status              # same checks as verify, never exits non-zero
```

## 5. Sync from the architecture repo

If you have access to the architecture repository (`hologram-architecture`):

```bash
# Auto-detect sibling directory
archon sync

# Or specify explicitly
archon sync --arch-root ~/work/hologram-architecture

# Or set once via env var
export ARCHON_ROOT=~/work/hologram-architecture
archon sync
```

This updates managed files and sections in your repo to reflect the latest architecture decisions.

## 6. Generate documents with AI

Interactively create specs, prompts, ADRs, or implementation plans:

```bash
archon generate
```

Follow the prompts to select document type, provide a title and description, and answer AI clarifying questions. The AI generates a complete document from your answers.

```bash
archon generate --doc-type spec --title "Auth Module" --dry-run  # non-interactive preview
archon generate --no-refine                                       # skip AI Q&A, generate directly
```

## 7. Manage ADRs and exceptions

```bash
# Create an Architecture Decision Record
archon adr new --title "Use YAML for all config files"
archon adr list

# Declare a policy exception
archon exception new --rule STR-003 --reason "Migrating in Q2" --expires 2026-06-01
archon exception list
```

## 8. AI-drafted decisions

```bash
archon decide --title "Should we use gRPC or REST for inter-service communication?"
archon decide --title "Error handling strategy" --dry-run
```

## Common workflows

### CI integration

```yaml
# .github/workflows/archon.yml
- name: Architecture conformance
  run: archon verify --strict --format json
```

### New repo setup (full)

```bash
mkdir my-new-repo && cd my-new-repo
cargo init --lib
archon init --arch-root ../hologram-architecture
archon verify
```

### Auth management

```bash
archon auth login                      # interactive provider selection
archon auth login --provider claude    # direct login
archon auth refresh                    # refresh expired token
archon auth status                     # check stored credentials
archon auth logout                     # remove credentials
```

## Environment variables

| Variable | Purpose |
|----------|---------|
| `ANTHROPIC_API_KEY` | Anthropic API key (highest priority AI backend) |
| `ARCHON_ROOT` | Path to the architecture repo |

## What's next

- Read the full [README](README.md) for architecture details
- Run `archon --help` or `archon <command> --help` for all options
