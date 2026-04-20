# tui-file-explorer — task runner
#
# Install just:     cargo install just
# Install nushell:  https://www.nushell.sh
# Usage:            just <task>   |   just --list

# Default task — show available commands
default:
    @just --list

# ── Prerequisites ─────────────────────────────────────────────────────────────

# Install required tools (just, git-cliff). Nu must be installed manually.
install-tools:
    @echo "Installing required tools..."
    @command -v just >/dev/null 2>&1 || cargo install just
    @command -v git-cliff >/dev/null 2>&1 || cargo install git-cliff
    @command -v nu >/dev/null 2>&1 && echo "✅ nu found" || echo "⚠ nu (nushell) not found. Install: https://www.nushell.sh"
    @echo "✅ Done!"

# Check git-cliff is available
check-git-cliff:
    @command -v git-cliff >/dev/null 2>&1 || { echo "❌ git-cliff not found. Run: just install-tools"; exit 1; }

# Check nu (nushell) is available
check-nu:
    @command -v nu >/dev/null 2>&1 || { echo "❌ nu (nushell) not found. Install: https://www.nushell.sh"; exit 1; }

# ── Build ─────────────────────────────────────────────────────────────────────

# Build (debug)
build:
    cargo build

# Build (release)
build-release:
    cargo build --release

# ── Code quality ──────────────────────────────────────────────────────────────

# Check code without building
check:
    cargo check

# Format code
fmt:
    cargo fmt

# Check formatting (CI-safe, no writes)
fmt-check:
    cargo fmt --check

# Run clippy
clippy:
    cargo clippy -- -D warnings

# Run all quality checks (fmt, clippy, test)
check-all: fmt-check clippy test
    @echo "✅ All checks passed!"

# ── Tests ─────────────────────────────────────────────────────────────────────

# Run tests
test:
    cargo test

# Run tests with all features
test-all:
    cargo test --all-features --all-targets

# Run Nu script tests
test-nu: check-nu
    nu scripts/tests/run_all.nu

# Run both Rust and Nu tests
test-all-nu: test-all test-nu
    @echo "✅ All Rust and Nu tests passed!"

# ── Documentation ─────────────────────────────────────────────────────────────

# Build and open docs
doc:
    cargo doc --no-deps --open

# Build docs (no open, for CI)
doc-build:
    cargo doc --no-deps --all-features

# ── Changelog ─────────────────────────────────────────────────────────────────

# Generate full changelog from all tags
changelog: check-git-cliff
    @echo "Generating full changelog..."
    git-cliff -o CHANGELOG.md
    @echo "✅ Changelog generated!"

# Preview unreleased changes (no file write)
changelog-preview: check-git-cliff
    @git-cliff --unreleased

# Prepend unreleased commits to existing changelog
changelog-unreleased: check-git-cliff
    @echo "Prepending unreleased commits..."
    git-cliff --unreleased --prepend CHANGELOG.md
    @echo "✅ Done!"

# Generate changelog for a specific version tag
changelog-version version: check-git-cliff
    @echo "Generating changelog for version {{ version }}..."
    git-cliff --tag v{{ version }} -o CHANGELOG.md
    @echo "✅ Changelog generated for v{{ version }}!"

# Regenerate full changelog from all history
changelog-update: check-git-cliff
    @echo "Regenerating complete changelog..."
    git-cliff --output CHANGELOG.md
    @echo "✅ Changelog updated!"

# ── Versioning & Release ──────────────────────────────────────────────────────

# Show current version
version: check-nu
    @nu scripts/version.nu

# Bump version interactively — prompts for confirmation, runs checks, commits and tags.

# Use `just release <version>` for fully automated non-interactive release.
bump version: check-all check-git-cliff check-nu
    nu scripts/bump_version.nu {{ version }}

# Run pre-publish readiness checks
release-check: check-nu
    nu scripts/check_publish.nu

# Publish to crates.io (dry run — no side-effects)
publish-dry:
    cargo publish --dry-run

# Publish to crates.io
publish: release-check
    cargo publish

# ── Full release workflows ────────────────────────────────────────────────────
# Full automated release to GitHub — bumps version, commits, tags, and pushes.
# This is the single command you run to cut a release:

# just release 0.2.0
release version: check-all check-git-cliff check-nu
    nu scripts/bump_version.nu --yes {{ version }}
    @echo "Pushing branch and tag to GitHub..."
    git push origin main
    git push origin v{{ version }}
    @echo "✅ Release v{{ version }} pushed — GitHub Actions will handle the rest."

# Full automated release to Gitea only.
release-gitea version: check-all check-git-cliff check-nu
    nu scripts/bump_version.nu --yes {{ version }}
    @echo "Pushing branch and tag to Gitea..."
    git push gitea main
    git push gitea v{{ version }}
    @echo "✅ Release v{{ version }} pushed to Gitea."

# Full automated release to Gitea Starscream only.
release-gitea-starscream version: check-all check-git-cliff check-nu
    nu scripts/bump_version.nu --yes {{ version }}
    @echo "Pushing branch and tag to Gitea Starscream..."
    git push gitea_starscream main
    git push gitea_starscream v{{ version }}
    @echo "✅ Release v{{ version }} pushed to Gitea Starscream."

# Full automated release to all remotes (continues on failure).
release-all version: check-all check-git-cliff check-nu
    #!/usr/bin/env sh
    set -e
    nu scripts/bump_version.nu --yes {{ version }}
    set +e
    echo "Pushing release v{{ version }} to all remotes…"
    failed=""
    git push --follow-tags origin main             || failed="$failed origin"
    git push --follow-tags gitea main              || failed="$failed gitea"
    git push --follow-tags gitea_starscream main   || failed="$failed gitea_starscream"
    if [ -n "$failed" ]; then
        echo "⚠️  Release v{{ version }} failed to push to:$failed"
    else
        echo "✅ Release v{{ version }} pushed to GitHub, Gitea, and Gitea Starscream!"
    fi

# Push the latest commit and all tags to every remote (no bump, continues on failure).
push-release-all: check-all
    #!/usr/bin/env sh
    failed=""
    git push --follow-tags origin main             || failed="$failed origin"
    git push --follow-tags gitea main              || failed="$failed gitea"
    git push --follow-tags gitea_starscream main   || failed="$failed gitea_starscream"
    if [ -n "$failed" ]; then
        echo "⚠️  Failed to push to:$failed"
    else
        echo "✅ Latest commit + tags pushed to all remotes."
    fi

# ── VHS / Demo GIFs ──────────────────────────────────────────────────────────

# Generate a single demo GIF (usage: just vhs basic)
vhs name:
    @echo "Recording {{ name }}.tape..."
    vhs examples/vhs/{{ name }}.tape
    @echo "✅ examples/vhs/generated/{{ name }}.gif created"

# Generate all demo GIFs (requires VHS: https://github.com/charmbracelet/vhs)
vhs-all:
    @command -v vhs >/dev/null 2>&1 || { echo "❌ vhs not found. Install: brew install vhs"; exit 1; }
    @echo "Building examples..."
    cargo build --example basic --example theme_switcher --example dual_pane --example options --example editor_picker --bin tfe
    @echo "Recording all tapes..."
    vhs examples/vhs/basic.tape
    vhs examples/vhs/search.tape
    vhs examples/vhs/sort.tape
    vhs examples/vhs/filter.tape
    vhs examples/vhs/file_ops.tape
    vhs examples/vhs/theme_switcher.tape
    vhs examples/vhs/pane_toggle.tape
    vhs examples/vhs/dual_pane.tape
    vhs examples/vhs/options.tape
    vhs examples/vhs/editor_picker.tape
    vhs examples/vhs/create_entries.tape
    @echo "✅ All GIFs generated in examples/vhs/generated/"

# ── Git remotes & pushing ────────────────────────────────────────────────────

# Show current git remotes
remotes:
    @git remote -v

# Add a Gitea remote (usage: just setup-gitea https://gitea.example.com/user/repo.git)
setup-gitea url:
    git remote add gitea {{ url }}
    @echo "✅ Gitea remote added! Test with: git push gitea main"

# Commit all staged changes
commit message:
    git add .
    git commit -m "{{ message }}"

# Pull from GitHub
pull:
    git pull origin main

# Pull from Gitea
pull-gitea:
    git pull gitea main

# Pull from Gitea Starscream
pull-gitea-starscream:
    git pull gitea_starscream main

# Pull from all remotes (continues on failure)
pull-all:
    #!/usr/bin/env sh
    failed=""
    git pull origin main             || failed="$failed origin"
    git pull gitea main              || failed="$failed gitea"
    git pull gitea_starscream main   || failed="$failed gitea_starscream"
    if [ -n "$failed" ]; then
        echo "⚠️  Failed to pull from:$failed"
    else
        echo "✅ Pulled from GitHub, Gitea, and Gitea Starscream!"
    fi

# Push to GitHub
push:
    git push origin main

# Push to Gitea
push-gitea:
    git push gitea main

# Push to Gitea Starscream
push-gitea-starscream:
    git push gitea_starscream main

# Push to all remotes (continues on failure)
push-all:
    #!/usr/bin/env sh
    failed=""
    git push origin main             || failed="$failed origin"
    git push gitea main              || failed="$failed gitea"
    git push gitea_starscream main   || failed="$failed gitea_starscream"
    if [ -n "$failed" ]; then
        echo "⚠️  Failed to push to:$failed"
    else
        echo "✅ Pushed to GitHub, Gitea, and Gitea Starscream!"
    fi

# Force-push to all remotes
push-all-force:
    #!/usr/bin/env sh
    failed=""
    git push --force origin main             || failed="$failed origin"
    git push --force gitea main              || failed="$failed gitea"
    git push --force gitea_starscream main   || failed="$failed gitea_starscream"
    if [ -n "$failed" ]; then
        echo "⚠️  Failed to force-push to:$failed"
    else
        echo "✅ Force-pushed to GitHub, Gitea, and Gitea Starscream!"
    fi

# Push tags to GitHub
push-tags:
    git push origin --tags

# Push tags to all remotes (continues on failure)
push-tags-all:
    #!/usr/bin/env sh
    failed=""
    git push origin --tags             || failed="$failed origin"
    git push gitea --tags              || failed="$failed gitea"
    git push gitea_starscream --tags   || failed="$failed gitea_starscream"
    if [ -n "$failed" ]; then
        echo "⚠️  Failed to push tags to:$failed"
    else
        echo "✅ Tags pushed to all remotes!"
    fi

# Force-sync Gitea from GitHub
sync-gitea:
    git push gitea main --force
    git push gitea --tags --force
    @echo "✅ Gitea synced!"

# Force-sync Gitea Starscream from GitHub
sync-gitea-starscream:
    git push gitea_starscream main --force
    git push gitea_starscream --tags --force
    @echo "✅ Gitea Starscream synced!"

# Force-sync all Gitea instances from GitHub (continues on failure)
sync-all-gitea:
    #!/usr/bin/env sh
    failed=""
    git push gitea main --force                  || failed="$failed gitea"
    git push gitea --tags --force                || failed="$failed gitea-tags"
    git push gitea_starscream main --force       || failed="$failed gitea_starscream"
    git push gitea_starscream --tags --force     || failed="$failed gitea_starscream-tags"
    if [ -n "$failed" ]; then
        echo "⚠️  Failed to sync:$failed"
    else
        echo "✅ All Gitea instances force-synced with GitHub."
    fi

# ── Misc ─────────────────────────────────────────────────────────────────────

# Update dependencies
update:
    cargo update

# Clean build artifacts
clean:
    cargo clean

# Show project info
info:
    @echo "Project:  tui-file-explorer"
    @echo "Version:  $(just version)"
    @echo "License:  MIT"
    @echo "Crate:    https://crates.io/crates/tui-file-explorer"

# View changelog
view-changelog:
    @cat CHANGELOG.md
