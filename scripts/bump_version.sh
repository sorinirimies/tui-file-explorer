#!/bin/bash
# Automated version bump script for tui-file-explorer
# Usage: ./scripts/bump_version.sh [--yes] <new_version>
# Example: ./scripts/bump_version.sh 0.2.0
#          ./scripts/bump_version.sh --yes 0.2.0   # skip confirmation (used by `just release`)

set -e

# Cross-platform in-place sed helper
# macOS (BSD sed) requires an empty extension: sed -i ''
# Linux (GNU sed) uses bare: sed -i
sed_inplace() {
    if [[ "$(uname)" == "Darwin" ]]; then
        sed -i '' "$@"
    else
        sed -i "$@"
    fi
}

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

# ── Argument parsing ──────────────────────────────────────────────────────────

AUTO_YES=false
NEW_VERSION=""

for arg in "$@"; do
    case "$arg" in
        -y|--yes) AUTO_YES=true ;;
        -*) echo -e "${RED}Error: Unknown flag: $arg${NC}"; exit 1 ;;
        *)  NEW_VERSION="$arg" ;;
    esac
done

if [ -z "$NEW_VERSION" ]; then
    echo -e "${RED}Error: Version number required${NC}"
    echo "Usage: $0 [--yes] <version>"
    echo "Example: $0 0.2.0"
    echo "         $0 --yes 0.2.0   # non-interactive"
    exit 1
fi

if ! [[ $NEW_VERSION =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9]+)?$ ]]; then
    echo -e "${RED}Error: Invalid version format${NC}"
    echo "Version must be in format: X.Y.Z or X.Y.Z-suffix (e.g., 0.2.0 or 0.2.0-beta.1)"
    exit 1
fi

echo -e "${CYAN}════════════════════════════════════════${NC}"
echo -e "${CYAN}  tui-file-explorer Version Bump${NC}"
echo -e "${CYAN}════════════════════════════════════════${NC}"
echo ""

# Extract current version robustly — handles any amount of whitespace around '='
CURRENT_VERSION=$(grep '^version[[:space:]]*=' Cargo.toml | head -1 | sed 's/^version[[:space:]]*=[[:space:]]*"\([^"]*\)".*/\1/')

if [ -z "$CURRENT_VERSION" ]; then
    echo -e "${RED}Error: Could not read current version from Cargo.toml${NC}"
    exit 1
fi

echo -e "Current version: ${YELLOW}${CURRENT_VERSION}${NC}"
echo -e "New version:     ${GREEN}${NEW_VERSION}${NC}"
echo ""

# Guard: abort early if the version is already at the requested value.
# Re-running the script for the same version produces duplicate commits and a
# tag that is already at origin, which means GitHub never sees a new tag push
# and the release workflow never fires.
if [ "$CURRENT_VERSION" = "$NEW_VERSION" ]; then
    echo -e "${RED}Error: Cargo.toml is already at version ${NEW_VERSION}.${NC}"
    echo -e "${YELLOW}  • If you need to re-release, delete the tag first:${NC}"
    echo -e "      git tag -d v${NEW_VERSION} && git push origin :refs/tags/v${NEW_VERSION}"
    echo -e "${YELLOW}  • Or bump to the next version instead.${NC}"
    exit 1
fi

# Guard: abort if a tag for this version already exists locally.
if git rev-parse "v${NEW_VERSION}" >/dev/null 2>&1; then
    echo -e "${RED}Error: Tag v${NEW_VERSION} already exists locally.${NC}"
    echo -e "${YELLOW}  Delete it first if you really want to recreate it:${NC}"
    echo -e "      git tag -d v${NEW_VERSION}"
    exit 1
fi

if $AUTO_YES; then
    echo -e "${CYAN}Running non-interactively (--yes passed).${NC}"
else
    read -p "Continue with version bump? (y/n) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo -e "${YELLOW}Aborted${NC}"
        exit 0
    fi
fi

echo ""
echo -e "${CYAN}Step 1/8: Updating Cargo.toml...${NC}"
# Use a POSIX character class so the pattern matches regardless of how many
# spaces surround '=' in the version line (e.g. "version      = \"0.1.0\"").
sed_inplace 's/^version[[:space:]]*=[[:space:]]*"[^"]*"/version      = "'"${NEW_VERSION}"'"/' Cargo.toml

# Verify the substitution actually took effect.
UPDATED_VERSION=$(grep '^version[[:space:]]*=' Cargo.toml | head -1 | sed 's/^version[[:space:]]*=[[:space:]]*"\([^"]*\)".*/\1/')
if [ "$UPDATED_VERSION" != "$NEW_VERSION" ]; then
    echo -e "${RED}✗ Failed to update version in Cargo.toml (got \"${UPDATED_VERSION}\").${NC}"
    echo -e "${YELLOW}  Check the version line format in Cargo.toml and update manually.${NC}"
    exit 1
fi
echo -e "${GREEN}✓ Cargo.toml updated (${CURRENT_VERSION} → ${NEW_VERSION})${NC}"

echo ""
echo -e "${CYAN}Step 2/8: Updating README.md badges...${NC}"
if grep -q "version-[0-9]*\.[0-9]*\.[0-9]*-blue" README.md 2>/dev/null; then
    sed_inplace -E "s/version-[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9]+)?-blue/version-${NEW_VERSION}-blue/" README.md
    echo -e "${GREEN}✓ README.md updated${NC}"
else
    echo -e "${YELLOW}⚠ No version badge found in README.md — skipping${NC}"
fi

echo ""
echo -e "${CYAN}Step 3/8: Updating Cargo.lock...${NC}"
cargo update -p tui-file-explorer
echo -e "${GREEN}✓ Cargo.lock updated${NC}"

echo ""
echo -e "${CYAN}Step 4/8: Running cargo fmt...${NC}"
cargo fmt
echo -e "${GREEN}✓ Code formatted${NC}"

echo ""
echo -e "${CYAN}Step 5/8: Running cargo clippy...${NC}"
if ! cargo clippy --all-targets --all-features -- -D warnings; then
    echo -e "${RED}✗ Clippy found issues. Please fix them before continuing.${NC}"
    exit 1
fi
echo -e "${GREEN}✓ Clippy passed${NC}"

echo ""
echo -e "${CYAN}Step 6/8: Running tests...${NC}"
if ! cargo test --all-features --all-targets; then
    echo -e "${RED}✗ Tests failed. Please fix them before continuing.${NC}"
    exit 1
fi
echo -e "${GREEN}✓ All tests passed${NC}"

echo ""
echo -e "${CYAN}Step 7/8: Generating CHANGELOG.md...${NC}"
if command -v git-cliff &> /dev/null; then
    git-cliff --tag "v${NEW_VERSION}" -o CHANGELOG.md
    echo -e "${GREEN}✓ Changelog generated${NC}"
else
    echo -e "${YELLOW}⚠ git-cliff not found — skipping changelog generation${NC}"
    echo -e "${YELLOW}  Install it with: cargo install git-cliff${NC}"
fi

echo ""
echo -e "${CYAN}Step 8/8: Creating git commit and tag...${NC}"

if git diff --quiet Cargo.toml Cargo.lock README.md CHANGELOG.md 2>/dev/null; then
    echo -e "${YELLOW}⚠ No changes to commit${NC}"
else
    git add Cargo.toml Cargo.lock README.md CHANGELOG.md

    git commit -m "chore: bump version to ${NEW_VERSION}

- Update version in Cargo.toml to ${NEW_VERSION}
- Update Cargo.lock
- Generate updated CHANGELOG.md"

    echo -e "${GREEN}✓ Changes committed${NC}"
fi

git tag -a "v${NEW_VERSION}" -m "Release v${NEW_VERSION}

Includes all changes documented in CHANGELOG.md for version ${NEW_VERSION}."
echo -e "${GREEN}✓ Tag v${NEW_VERSION} created${NC}"

echo ""
echo -e "${CYAN}════════════════════════════════════════${NC}"
echo -e "${GREEN}✓ Version bump complete! 🚀${NC}"
echo -e "${CYAN}════════════════════════════════════════${NC}"
echo ""
echo -e "${YELLOW}Next steps:${NC}"
echo -e "  1. Review the changes:"
echo -e "     ${CYAN}git show${NC}"
echo -e ""
echo -e "  2. Push to GitHub (triggers the release workflow):"
echo -e "     ${CYAN}git push origin main${NC}"
echo -e "     ${CYAN}git push origin v${NEW_VERSION}${NC}"
echo -e ""
echo -e "  3. (Optional) Push to Gitea as well:"
echo -e "     ${CYAN}git push gitea main && git push gitea v${NEW_VERSION}${NC}"
echo -e ""
echo -e "  4. The GitHub Actions release workflow will publish to crates.io automatically"
echo -e "     once CRATES_IO_TOKEN is set in repository secrets."
echo ""
