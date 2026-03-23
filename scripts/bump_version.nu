#!/usr/bin/env nu
# Automated version bump script for tui-file-explorer
# Usage: nu scripts/bump_version.nu [--yes] <new_version>
# Example: nu scripts/bump_version.nu 0.2.0
#          nu scripts/bump_version.nu --yes 0.2.0   # skip confirmation

def main [
    new_version: string,  # New version in X.Y.Z or X.Y.Z-suffix format
    --yes (-y),           # Skip confirmation prompt (non-interactive)
] {
    let red    = (ansi red)
    let green  = (ansi green)
    let yellow = (ansi yellow)
    let cyan   = (ansi cyan)
    let reset  = (ansi reset)

    # ── Validate version format ───────────────────────────────────────────────
    if not ($new_version =~ '^[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.]+)?$') {
        error make { msg: $"($red)Error: Invalid version format($reset)
Version must be in format: X.Y.Z or X.Y.Z-suffix \(e.g., 0.2.0 or 0.2.0-beta.1\)" }
    }

    print $"($cyan)════════════════════════════════════════($reset)"
    print $"($cyan)  tui-file-explorer Version Bump($reset)"
    print $"($cyan)════════════════════════════════════════($reset)"
    print ""

    # ── Read current version from Cargo.toml ─────────────────────────────────
    let cargo_lines = (open Cargo.toml --raw | lines)

    let current_version = (
        $cargo_lines
        | where { |line| $line =~ '^version\s*=' }
        | first
        | parse --regex 'version\s*=\s*"(?P<ver>[^"]+)"'
        | get ver
        | first
    )

    if ($current_version | is-empty) {
        error make { msg: $"($red)Error: Could not read current version from Cargo.toml($reset)" }
    }

    print $"Current version: ($yellow)($current_version)($reset)"
    print $"New version:     ($green)($new_version)($reset)"
    print ""

    # ── Guard: already at requested version ──────────────────────────────────
    if $current_version == $new_version {
        error make { msg: $"($red)Error: Cargo.toml is already at version ($new_version).($reset)
($yellow)  If you need to re-release, delete the tag first:($reset)
      git tag -d v($new_version) && git push origin :refs/tags/v($new_version)
($yellow)  Or bump to the next version instead.($reset)" }
    }

    # ── Guard: tag already exists locally ────────────────────────────────────
    let tag_name = $"v($new_version)"
    let existing_tags = (git tag | lines)
    if ($existing_tags | any { |t| $t == $tag_name }) {
        error make { msg: $"($red)Error: Tag ($tag_name) already exists locally.($reset)
($yellow)  Delete it first if you really want to recreate it:($reset)
      git tag -d ($tag_name)" }
    }

    # ── Confirmation ─────────────────────────────────────────────────────────
    if $yes {
        print $"($cyan)Running non-interactively \(--yes passed\).($reset)"
    } else {
        let reply = (input "Continue with version bump? (y/n) ")
        if not ($reply =~ '^[Yy]') {
            print $"($yellow)Aborted($reset)"
            return
        }
    }

    print ""

    # ── Step 1: Update Cargo.toml ─────────────────────────────────────────────
    print $"($cyan)Step 1/8: Updating Cargo.toml...($reset)"

    let updated_cargo = (
        $cargo_lines
        | each { |line|
            if ($line =~ '^version\s*=\s*"[^"]*"') {
                $'version      = "($new_version)"'
            } else {
                $line
            }
        }
        | str join "\n"
    )
    $updated_cargo | save --force Cargo.toml

    # Verify the substitution took effect
    let verify_version = (
        open Cargo.toml --raw
        | lines
        | where { |line| $line =~ '^version\s*=' }
        | first
        | parse --regex 'version\s*=\s*"(?P<ver>[^"]+)"'
        | get ver
        | first
    )
    if $verify_version != $new_version {
        error make { msg: $"($red)Failed to update version in Cargo.toml \(got ($verify_version)\).($reset)
($yellow)  Check the version line format in Cargo.toml and update manually.($reset)" }
    }
    print $"($green)✓ Cargo.toml updated \(($current_version) → ($new_version)\)($reset)"

    # ── Step 2: Update README.md badges ──────────────────────────────────────
    print ""
    print $"($cyan)Step 2/8: Updating README.md badges...($reset)"

    if ("README.md" | path exists) {
        let readme = (open README.md --raw)
        if ($readme =~ 'version-[0-9]+\.[0-9]+\.[0-9]+-blue') {
            let updated_readme = (
                $readme
                | str replace --all --regex 'version-[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9]+)?-blue' $"version-($new_version)-blue"
            )
            $updated_readme | save --force README.md
            print $"($green)✓ README.md updated($reset)"
        } else {
            print $"($yellow)⚠ No version badge found in README.md — skipping($reset)"
        }
    } else {
        print $"($yellow)⚠ README.md not found — skipping($reset)"
    }

    # ── Step 3: Update Cargo.lock ─────────────────────────────────────────────
    print ""
    print $"($cyan)Step 3/8: Updating Cargo.lock...($reset)"
    run-external "cargo" "update" "-p" "tui-file-explorer"
    print $"($green)✓ Cargo.lock updated($reset)"

    # ── Step 4: cargo fmt ─────────────────────────────────────────────────────
    print ""
    print $"($cyan)Step 4/8: Running cargo fmt...($reset)"
    run-external "cargo" "fmt"
    print $"($green)✓ Code formatted($reset)"

    # ── Step 5: cargo clippy ──────────────────────────────────────────────────
    print ""
    print $"($cyan)Step 5/8: Running cargo clippy...($reset)"
    let clippy = (do { run-external "cargo" "clippy" "--all-targets" "--all-features" "--" "-D" "warnings" } | complete)
    if $clippy.exit_code != 0 {
        error make { msg: $"($red)✗ Clippy found issues. Please fix them before continuing.($reset)" }
    }
    print $"($green)✓ Clippy passed($reset)"

    # ── Step 6: cargo test ────────────────────────────────────────────────────
    print ""
    print $"($cyan)Step 6/8: Running tests...($reset)"
    let tests = (do { run-external "cargo" "test" "--all-features" "--all-targets" } | complete)
    if $tests.exit_code != 0 {
        error make { msg: $"($red)✗ Tests failed. Please fix them before continuing.($reset)" }
    }
    print $"($green)✓ All tests passed($reset)"

    # ── Step 7: Generate CHANGELOG.md ────────────────────────────────────────
    print ""
    print $"($cyan)Step 7/8: Generating CHANGELOG.md...($reset)"
    if (which git-cliff | length) > 0 {
        run-external "git-cliff" "--tag" $tag_name "-o" "CHANGELOG.md"
        print $"($green)✓ Changelog generated($reset)"
    } else {
        print $"($yellow)⚠ git-cliff not found — skipping changelog generation($reset)"
        print $"($yellow)  Install it with: cargo install git-cliff($reset)"
    }

    # ── Step 8: Git commit and tag ────────────────────────────────────────────
    print ""
    print $"($cyan)Step 8/8: Creating git commit and tag...($reset)"

    let diff = (do { run-external "git" "diff" "--quiet" "Cargo.toml" "Cargo.lock" "README.md" "CHANGELOG.md" } | complete)
    if $diff.exit_code == 0 {
        print $"($yellow)⚠ No changes to commit($reset)"
    } else {
        run-external "git" "add" "Cargo.toml" "Cargo.lock" "README.md" "CHANGELOG.md"
        let commit_msg = $"chore: bump version to ($new_version)

- Update version in Cargo.toml to ($new_version)
- Update Cargo.lock
- Generate updated CHANGELOG.md"
        run-external "git" "commit" "-m" $commit_msg
        print $"($green)✓ Changes committed($reset)"
    }

    let tag_msg = $"Release ($tag_name)

Includes all changes documented in CHANGELOG.md for version ($new_version)."
    run-external "git" "tag" "-a" $tag_name "-m" $tag_msg
    print $"($green)✓ Tag ($tag_name) created($reset)"

    # ── Summary ───────────────────────────────────────────────────────────────
    print ""
    print $"($cyan)════════════════════════════════════════($reset)"
    print $"($green)✓ Version bump complete! 🚀($reset)"
    print $"($cyan)════════════════════════════════════════($reset)"
    print ""
    print $"($yellow)Next steps:($reset)"
    print  "  1. Review the changes:"
    print $"     ($cyan)git show($reset)"
    print  ""
    print  "  2. Push to GitHub (triggers the release workflow):"
    print $"     ($cyan)git push origin main($reset)"
    print $"     ($cyan)git push origin ($tag_name)($reset)"
    print  ""
    print  "  3. (Optional) Push to Gitea as well:"
    print $"     ($cyan)git push gitea main && git push gitea ($tag_name)($reset)"
    print  ""
    print  "  4. The GitHub Actions release workflow will publish to crates.io automatically"
    print  "     once CRATES_IO_TOKEN is set in repository secrets."
    print ""
}
