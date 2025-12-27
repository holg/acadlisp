#!/usr/bin/env bash
# File: /Users/htr/Documents/develeop/rust/acadlisp/scripts/test-ci-locally.sh

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${YELLOW}=== Testing CI Workflow Locally ===${NC}\n"

# Run cargo fmt check
echo -e "${YELLOW}Step 1: Running cargo fmt check...${NC}"
if cargo fmt -- --check; then
    echo -e "${GREEN}✓ cargo fmt passed${NC}\n"
else
    echo -e "${RED}✗ cargo fmt failed${NC}"
    echo -e "${YELLOW}Run 'cargo fmt' to fix formatting issues${NC}\n"
    exit 1
fi

# Run clippy
echo -e "${YELLOW}Step 2: Running clippy...${NC}"
if cargo clippy --all-targets -- -D warnings; then
    echo -e "${GREEN}✓ clippy passed${NC}\n"
else
    echo -e "${RED}✗ clippy failed${NC}\n"
    exit 1
fi

# Run cargo build
echo -e "${YELLOW}Step 3: Running cargo build...${NC}"
if cargo build; then
    echo -e "${GREEN}✓ build passed${NC}\n"
else
    echo -e "${RED}✗ build failed${NC}\n"
    exit 1
fi

# Run cargo doc
echo -e "${YELLOW}Step 4: Running cargo doc...${NC}"
if RUSTDOCFLAGS="-D warnings" cargo doc --document-private-items --no-deps; then
    echo -e "${GREEN}✓ doc generation passed${NC}\n"
else
    echo -e "${RED}✗ doc generation failed${NC}\n"
    exit 1
fi

# Additional checks
echo -e "${YELLOW}Step 5: Running additional checks...${NC}"

# Check cargo-sort
if command -v cargo-sort &> /dev/null; then
    echo "Running cargo-sort check..."
    if cargo-sort -cw; then
        echo -e "${GREEN}✓ cargo-sort passed${NC}"
    else
        echo -e "${RED}✗ cargo-sort failed${NC}"
        echo -e "${YELLOW}Run 'cargo-sort -w' to fix Cargo.toml sorting${NC}"
        exit 1
    fi
else
    echo -e "${YELLOW}⚠ cargo-sort not installed, skipping Cargo.toml sort check${NC}"
    echo -e "${YELLOW}Install with: cargo install cargo-sort${NC}"
fi

# Check taplo
if command -v taplo &> /dev/null; then
    echo "Running taplo format check..."
    if taplo format --check; then
        echo -e "${GREEN}✓ taplo passed${NC}"
    else
        echo -e "${RED}✗ taplo failed${NC}"
        echo -e "${YELLOW}Run 'taplo format' to fix TOML formatting${NC}"
        exit 1
    fi
else
    echo -e "${YELLOW}⚠ taplo not installed, skipping TOML format check${NC}"
    echo -e "${YELLOW}Install with: cargo install taplo-cli${NC}"
fi

# Check cargo-deny
if command -v cargo-deny &> /dev/null; then
    echo "Running cargo-deny check..."
    if cargo-deny check bans licenses sources --hide-inclusion-graph --show-stats; then
        echo -e "${GREEN}✓ cargo-deny passed${NC}"
    else
        echo -e "${RED}✗ cargo-deny failed${NC}"
        exit 1
    fi
else
    echo -e "${YELLOW}⚠ cargo-deny not installed, skipping dependency check${NC}"
    echo -e "${YELLOW}Install with: cargo install cargo-deny${NC}"
fi

# Run Rust tests
echo -e "\n${YELLOW}Step 6: Running Rust tests...${NC}"
if cargo test; then
    echo -e "${GREEN}✓ tests passed${NC}\n"
else
    echo -e "${RED}✗ tests failed${NC}\n"
    exit 1
fi

# Build WASM target with Trunk
echo -e "${YELLOW}Step 7: Building WASM (Trunk)...${NC}"
if command -v trunk &> /dev/null && command -v wasm-bindgen &> /dev/null; then
    echo "Building with Trunk..."
    # Use homebrew's wasm-opt if available (newer version handles WASM features better)
    WASM_OPT_PATH=""
    if [[ -x "/opt/homebrew/opt/binaryen/bin/wasm-opt" ]]; then
        WASM_OPT_PATH="/opt/homebrew/opt/binaryen/bin"
    fi
    if PATH="${WASM_OPT_PATH:-}:$PATH" trunk build --release; then
        echo -e "${GREEN}✓ WASM build passed${NC}\n"
    else
        echo -e "${RED}✗ WASM build failed${NC}\n"
        exit 1
    fi
else
    echo -e "${YELLOW}⚠ trunk or wasm-bindgen-cli not installed, skipping WASM build${NC}"
    echo -e "${YELLOW}Install with:${NC}"
    echo -e "${YELLOW}  cargo install trunk${NC}"
    echo -e "${YELLOW}  cargo install wasm-bindgen-cli${NC}"
    echo -e "${YELLOW}  rustup target add wasm32-unknown-unknown${NC}"
fi

# Check for uncommitted changes
echo -e "${YELLOW}Step 8: Checking for uncommitted changes...${NC}"
if [[ -n $(git status --porcelain) ]]; then
    echo -e "${YELLOW}⚠ You have uncommitted changes:${NC}"
    git status --short
    echo -e "${YELLOW}Consider committing or stashing changes before publishing${NC}\n"
else
    echo -e "${GREEN}✓ No uncommitted changes${NC}\n"
fi

# Check if on main branch
echo -e "${YELLOW}Step 9: Checking git branch...${NC}"
current_branch=$(git branch --show-current)
if [[ "$current_branch" != "main" ]]; then
    echo -e "${YELLOW}⚠ You are on branch '$current_branch', not 'main'${NC}"
    echo -e "${YELLOW}Consider switching to main branch before publishing${NC}\n"
else
    echo -e "${GREEN}✓ On main branch${NC}\n"
fi

echo -e "\n${GREEN}=== All CI checks passed! ===${NC}"
echo -e "${GREEN}Your code is ready to be pushed.${NC}"
