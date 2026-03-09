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

# ─── Release (requires git-cliff + gh CLI) ─────────────────────────────────

# Create a release with an explicit version: just release 0.2.0
release VERSION:
    #!/usr/bin/env bash
    set -euo pipefail

    # Preflight checks
    command -v git-cliff >/dev/null 2>&1 || { echo "error: git-cliff not found — cargo install git-cliff"; exit 1; }
    command -v gh >/dev/null 2>&1 || { echo "error: gh CLI not found — https://cli.github.com"; exit 1; }
    gh auth status >/dev/null 2>&1 || { echo "error: gh not authenticated — run: gh auth login"; exit 1; }

    TAG="v{{VERSION}}"
    echo "Releasing $TAG..."

    # Run CI checks
    just ci

    # Update version in Cargo.toml
    sed -i.bak 's/^version = ".*"/version = "{{VERSION}}"/' Cargo.toml && rm Cargo.toml.bak
    cargo check

    # Generate changelog
    touch CHANGELOG.md
    git cliff --tag "$TAG" --unreleased --prepend CHANGELOG.md

    # Generate release notes (just the latest version, for gh release)
    git cliff --tag "$TAG" --unreleased > RELEASE_NOTES.md

    # Commit, tag, push (Cargo.lock may be gitignored for binaries)
    git add Cargo.toml CHANGELOG.md
    git add Cargo.lock 2>/dev/null || true
    git commit -m "chore(release): $TAG"
    git tag -a "$TAG" -m "Release $TAG"
    git push origin main "$TAG"

    # Create GitHub release (binaries are built by the release workflow)
    gh release create "$TAG" \
        --title "$TAG" \
        --notes-file RELEASE_NOTES.md

    rm -f RELEASE_NOTES.md
    echo "✓ Released $TAG — GitHub Actions will build and attach binaries"

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
