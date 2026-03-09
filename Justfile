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

# Create a release with an explicit version: just release 0.2.0
release VERSION:
    #!/usr/bin/env bash
    set -euo pipefail
    just ci
    # Update version in Cargo.toml
    sed -i.bak 's/^version = ".*"/version = "{{VERSION}}"/' Cargo.toml && rm Cargo.toml.bak
    cargo check
    # Generate changelog
    touch CHANGELOG.md
    git cliff --tag "v{{VERSION}}" --unreleased --prepend CHANGELOG.md
    # Commit, tag, push (Cargo.lock may be gitignored for binaries)
    git add Cargo.toml CHANGELOG.md
    git add Cargo.lock 2>/dev/null || true
    git commit -m "chore(release): v{{VERSION}}"
    git tag "v{{VERSION}}"
    git push --follow-tags

# Auto-detect next version from conventional commits and release
release-auto:
    #!/usr/bin/env bash
    set -euo pipefail
    # Determine bump type from commits since last tag
    LAST_TAG=$(git describe --tags --abbrev=0 2>/dev/null || echo "v0.0.0")
    echo "Last tag: $LAST_TAG"
    # Check for breaking changes, features, or fixes
    COMMITS=$(git log "${LAST_TAG}..HEAD" --oneline 2>/dev/null || git log --oneline)
    if echo "$COMMITS" | grep -qiE '^[a-f0-9]+ .*(BREAKING|!:)'; then
        BUMP="major"
    elif echo "$COMMITS" | grep -qiE '^[a-f0-9]+ feat'; then
        BUMP="minor"
    else
        BUMP="patch"
    fi
    # Parse current version
    CURRENT=$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/')
    IFS='.' read -r MAJOR MINOR PATCH <<< "$CURRENT"
    case "$BUMP" in
        major) MAJOR=$((MAJOR + 1)); MINOR=0; PATCH=0 ;;
        minor) MINOR=$((MINOR + 1)); PATCH=0 ;;
        patch) PATCH=$((PATCH + 1)) ;;
    esac
    NEXT="${MAJOR}.${MINOR}.${PATCH}"
    echo "Bump: $BUMP ($CURRENT → $NEXT)"
    just release "$NEXT"

# Generate changelog without releasing
changelog:
    git cliff --unreleased

clean:
    cargo clean
