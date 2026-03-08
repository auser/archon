# AGENTS.md

This document provides guidance for automated agents operating in **`{{name}}`**.

---

## Repository Purpose

`{{name}}` is a **{{role}}** repository in the Hologram ecosystem.

Standards version: `{{standards_version}}`

---

## Repository Structure

```
specs/
  docs/         — project documentation
  adrs/         — architecture decision records
```

---

## Rules for Agents

1. Follow the architecture standards defined in `hologram-architecture`
2. Do not modify files outside this repository unless explicitly instructed
3. Run `cargo clippy -- -D warnings` before committing Rust changes
4. Use the `hologram-` prefix for all crate names

---

<!-- ARCHON:MANAGED:BEGIN -->
This section is managed by archon. Do not edit manually.
<!-- ARCHON:MANAGED:END -->
