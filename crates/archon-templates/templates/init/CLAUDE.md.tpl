# CLAUDE.md

This file provides context for Claude Code when working in **`{{name}}`**.

## Project Overview

- **Name**: {{name}}
- **Role**: {{role}}
- **Type**: {{repo_type}}
- **Standards Version**: {{standards_version}}

## Architecture

This repository follows the Hologram ecosystem architecture standards.
See `specs/docs/architecture.md` for project-specific architecture details.

## Development

- Run tests: `cargo test`
- Check lints: `cargo clippy -- -D warnings`
- Format code: `cargo fmt`

## Conventions

- Use `hologram-` prefix for all crate names
- Follow architecture decisions from `hologram-architecture`
- Document significant decisions as ADRs in `specs/adrs/`
