# archon — architecture governance tool

# Default: list available recipes
default:
    @just --list

# ─── Development ──────────────────────────────────────────────────────────────

# Run all workspace tests
test:
    cargo test --workspace

# Run clippy with warnings as errors
clippy:
    cargo clippy --workspace --all-targets -- -D warnings

# Format all code
fmt:
    cargo fmt --all

# Check formatting without modifying
fmt-check:
    cargo fmt --all -- --check

# Full CI pipeline: format check, clippy, tests
ci: fmt-check clippy test

# ─── Build ────────────────────────────────────────────────────────────────────

# Build all crates in debug mode
build:
    cargo build --workspace

# Build archon CLI in release mode
build-release:
    cargo build --release -p archon-cli

# Build CLI and symlink to ~/.local/bin/archon
build-cli: build-release
    mkdir -p ~/.local/bin
    ln -sf "$(pwd)/target/release/archon" ~/.local/bin/archon
    @echo ""
    @echo "✓ Symlinked: ~/.local/bin/archon → $(pwd)/target/release/archon"
    @echo "  Ensure ~/.local/bin is in your PATH"

# Install archon into ~/.local/bin (symlink for local dev)
install: build-release
    mkdir -p ~/.local/bin
    ln -sf "$(pwd)/target/release/archon" ~/.local/bin/archon
    @echo ""
    @echo "✓ Symlinked: ~/.local/bin/archon → $(pwd)/target/release/archon"
    @echo ""
    @echo "Run from any directory:"
    @echo "  archon init --profile rust-workspace"
    @echo "  archon verify"
    @echo "  archon status"

# ─── Run ──────────────────────────────────────────────────────────────────────

# Run archon with arguments: just run init --profile cli-tool
run *args:
    cargo run -p archon-cli -- {{args}}

# Initialize a repo (shortcut): just init --profile runtime-system
init *flags:
    cargo run -p archon-cli -- init {{flags}}

# Verify conformance (shortcut)
verify *flags:
    cargo run -p archon-cli -- verify {{flags}}

# Show status (shortcut)
status *flags:
    cargo run -p archon-cli -- status {{flags}}

# ─── Release ──────────────────────────────────────────────────────────────────

# Cut a release with automatic version bump (requires git-cliff)
release-auto:
    #!/usr/bin/env bash
    set -euo pipefail
    echo "==> Preparing automatic release"
    # 1. Quality gates
    just ci
    # 2. Determine next version from conventional commits
    NEXT_VERSION=$(git cliff --bumped-version | sed 's/^v//')
    echo "==> Auto-detected next version: $NEXT_VERSION"
    # 3. Update version in workspace Cargo.toml
    sed -i.bak "s/^version = \".*\"/version = \"$NEXT_VERSION\"/" Cargo.toml
    rm Cargo.toml.bak
    cargo update
    git add Cargo.toml Cargo.lock
    # 4. Generate changelog and create tag
    touch CHANGELOG.md
    git cliff --tag "v$NEXT_VERSION" --unreleased --prepend CHANGELOG.md
    git add CHANGELOG.md
    git commit -m "chore(release): prepare v$NEXT_VERSION"
    git tag "v$NEXT_VERSION"
    # 5. Push commits and tags
    git push --follow-tags
    echo "==> Release v$NEXT_VERSION complete."

# Cut a release with specific version: just release 2.0.0
release VERSION:
    #!/usr/bin/env bash
    set -euo pipefail
    echo "==> Preparing release v{{VERSION}}"
    # 1. Quality gates
    just ci
    # 2. Update version in workspace Cargo.toml
    sed -i.bak 's/^version = ".*"/version = "{{VERSION}}"/' Cargo.toml
    rm Cargo.toml.bak
    cargo update
    git add Cargo.toml Cargo.lock
    # 3. Generate changelog and create tag
    touch CHANGELOG.md
    git cliff --tag "v{{VERSION}}" --unreleased --prepend CHANGELOG.md
    git add CHANGELOG.md
    git commit -m "chore(release): prepare v{{VERSION}}"
    git tag "v{{VERSION}}"
    # 4. Push commits and tags
    git push --follow-tags
    echo "==> Release v{{VERSION}} complete."

# ─── Utility ──────────────────────────────────────────────────────────────────

# Show workspace crate dependency graph
deps:
    cargo tree --workspace --depth 1

# Clean build artifacts
clean:
    cargo clean
