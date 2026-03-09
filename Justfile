# archon — architecture governance tool

default:
    @just --list

# ─── Dev ─────────────────────────────────────────────────────────────────────

test:
    cargo test

clippy:
    cargo clippy --all-targets -- -D warnings

fmt:
    cargo fmt

ci: fmt clippy test

# ─── Build & Install ─────────────────────────────────────────────────────────

build:
    cargo build

install:
    cargo build --release
    mkdir -p ~/.local/bin
    ln -sf "$(pwd)/target/release/archon" ~/.local/bin/archon
    @echo "✓ ~/.local/bin/archon installed"

# ─── Run shortcuts ───────────────────────────────────────────────────────────

run *args:
    cargo run -- {{args}}

init *flags:
    cargo run -- init {{flags}}

verify *flags:
    cargo run -- verify {{flags}}

scan *flags:
    cargo run -- scan {{flags}}

sync *flags:
    cargo run -- sync {{flags}}

describe *args:
    cargo run -- describe {{args}}

# ─── Release (requires git-cliff) ───────────────────────────────────────────

release VERSION:
    #!/usr/bin/env bash
    set -euo pipefail
    just ci
    sed -i.bak 's/^version = ".*"/version = "{{VERSION}}"/' Cargo.toml && rm Cargo.toml.bak
    cargo update
    touch CHANGELOG.md
    git cliff --tag "v{{VERSION}}" --unreleased --prepend CHANGELOG.md
    git add Cargo.toml Cargo.lock CHANGELOG.md
    git commit -m "chore(release): v{{VERSION}}"
    git tag "v{{VERSION}}"
    git push --follow-tags

clean:
    cargo clean
