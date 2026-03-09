# Quickstart

Get archon running in under 5 minutes.

## Install

```bash
cargo install --path .
# or: just install
```

## 1. Initialize a repo

```bash
cd ~/work/my-project
archon init
```

The interactive wizard prompts for name, description, role, and dependencies. It auto-detects Cargo.toml workspace members and generates default rules.

This creates:

| File | Purpose |
|------|---------|
| `archon.yaml` | Repo identity — role, dependencies, rules |
| `CLAUDE.md` | Context injection target for AI agents |

Options:

```bash
archon init --registry ../archon-registry   # discover sibling repos
archon init --owner "@my-team"                  # set owner
```

Non-interactive mode activates automatically when stdin is not a TTY (e.g. in CI).

## 2. Extract your public API

```bash
archon scan
```

Parses your crate source with `syn`, extracts all `pub` types/traits/functions, and writes `.archon/broadcast.yaml` — a machine-readable API surface. If a registry is accessible, also copies the broadcast there.

Tag items with contracts using doc comments:

```rust
/// @contract(my-contract-name)
pub struct MyType { ... }
```

## 3. Assemble the ecosystem graph

Run from the parent directory containing all repos:

```bash
archon assemble --root .. --distribute
```

This collects all `archon.yaml` manifests, builds `graph.yaml`, generates per-repo AI context, and (with `--distribute`) injects context into each repo's `CLAUDE.md`.

## 4. Validate

```bash
# Check graph consistency (missing deps, cycles)
archon check

# Run rules defined in archon.yaml
archon verify
```

```bash
archon check --format json    # for CI
archon verify --format json   # for CI
```

## 5. Set up an architecture repo

An architecture repo is a dedicated repo that holds the graph and broadcasts for the whole ecosystem. Create one alongside your project repos:

```bash
mkdir hologram-architecture && cd hologram-architecture
git init
mkdir -p archon-registry/broadcasts
```

Directory layout:

```
~/work/
  hologram/                 # core repo
  hologram-ai/              # extension
  hologram-sandbox/         # extension
  hologram-architecture/    # architecture repo ← you are here
```

Bootstrap the entire ecosystem in one command — this creates `archon.yaml` for every sibling repo that has a `Cargo.toml` but no manifest yet:

```bash
archon assemble --root .. --bootstrap --registry archon-registry --distribute
```

This will:
1. Scan sibling directories for Rust projects without manifests
2. Auto-generate a minimal `archon.yaml` for each (name from Cargo.toml, default rules)
3. Assemble the full dependency graph
4. Distribute AI context to each repo's `CLAUDE.md`

After bootstrap, use `describe` to configure everything in plain English instead of editing YAML by hand:

```bash
archon describe --root .. "hologram is the core runtime. hologram-ai extends it with AI inference. hologram-sandbox provides sandboxed execution depending on hologram. archon is a developer tool."
```

AI reads your description, updates each repo's role, dependencies, and description, then writes the manifests. Use `--dry-run` to preview first.

Then re-assemble:

```bash
archon assemble --root .. --registry archon-registry --distribute
archon check
```

You can run `describe` again anytime to refine — it's incremental:

```bash
archon describe --root .. "hologram-sandbox also depends on hologram-ai for model scoring"
```

When a developer adds a new repo later:

```bash
cd ~/work/new-repo
archon init --registry ../hologram-architecture/archon-registry
archon scan --registry ../hologram-architecture/archon-registry
```

Then the architecture maintainer re-runs `assemble --distribute` to update context for everyone.

## CI integration

```yaml
# .github/workflows/archon.yml
- name: Architecture conformance
  run: |
    archon check --format json
    archon verify --format json
```

## Environment variables

| Variable | Purpose |
|----------|---------|
| `ANTHROPIC_API_KEY` | API key for AI features (optional) |

## What's next

- Read the [README](README.md) for full command reference
- Run `archon --help` or `archon <command> --help` for all options
