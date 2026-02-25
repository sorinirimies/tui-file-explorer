#!/bin/bash
# Pre-publish readiness check for tui-file-explorer
# Run this before 'cargo publish' to catch problems early.

echo "Checking tui-file-explorer for publish readiness..."
echo ""

GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m'

errors=0

# 1. Formatting
echo -n "Checking code formatting... "
if cargo fmt -- --check > /dev/null 2>&1; then
    echo -e "${GREEN}✓${NC}"
else
    echo -e "${RED}✗  (run: cargo fmt)${NC}"
    errors=$((errors + 1))
fi

# 2. Clippy
echo -n "Checking clippy... "
if cargo clippy --lib -- -D warnings > /dev/null 2>&1; then
    echo -e "${GREEN}✓${NC}"
else
    echo -e "${RED}✗  (run: cargo clippy -- -D warnings)${NC}"
    errors=$((errors + 1))
fi

# 3. Tests
echo -n "Running tests... "
if cargo test --all-features > /dev/null 2>&1; then
    echo -e "${GREEN}✓${NC}"
else
    echo -e "${RED}✗  (run: cargo test --all-features)${NC}"
    errors=$((errors + 1))
fi

# 4. Documentation
echo -n "Building documentation... "
if cargo doc --no-deps > /dev/null 2>&1; then
    echo -e "${GREEN}✓${NC}"
else
    echo -e "${RED}✗  (run: cargo doc --no-deps)${NC}"
    errors=$((errors + 1))
fi

# 5. Required files
echo -n "Checking required files... "
missing=0
for file in README.md LICENSE Cargo.toml CHANGELOG.md; do
    if [ ! -f "$file" ]; then
        echo -e "${RED}Missing: $file${NC}"
        missing=$((missing + 1))
    fi
done
if [ $missing -eq 0 ]; then
    echo -e "${GREEN}✓${NC}"
else
    errors=$((errors + 1))
fi

# 6. Dry run
echo -n "Cargo publish dry-run... "
if cargo publish --dry-run > /dev/null 2>&1; then
    echo -e "${GREEN}✓${NC}"
else
    echo -e "${RED}✗  (run: cargo publish --dry-run for details)${NC}"
    errors=$((errors + 1))
fi

echo ""
if [ $errors -eq 0 ]; then
    echo -e "${GREEN}✓ All checks passed! Ready to publish.${NC}"
    echo ""
    echo "Run: cargo publish"
    exit 0
else
    echo -e "${RED}✗ $errors check(s) failed. Please fix before publishing.${NC}"
    exit 1
fi
