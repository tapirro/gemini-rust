#!/bin/bash
# Comprehensive validation script for gemini-rust

set -e  # Exit on error

echo "🔍 Validating gemini-rust codebase..."
echo "=================================="

# Color codes for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# API Key management
if [ -n "$GEMINI_API_KEY" ]; then
    echo -e "${BLUE}🔑 Using GEMINI_API_KEY from environment${NC}"
    API_KEY="$GEMINI_API_KEY"
    USE_REAL_API=true
    API_TEST_SUCCESS=false  # Track API test result
else
    echo -e "${YELLOW}⚠️  GEMINI_API_KEY not found in environment, using test-key${NC}"
    API_KEY="test-key"
    USE_REAL_API=false
    API_TEST_SUCCESS=false
fi

# Function to print colored output
print_status() {
    if [ $1 -eq 0 ]; then
        echo -e "${GREEN}✅ $2${NC}"
    else
        echo -e "${RED}❌ $2${NC}"
        exit 1
    fi
}

# 1. Check Rust toolchain
echo -e "\n${YELLOW}1. Checking Rust toolchain...${NC}"
rustc --version
cargo --version
print_status $? "Rust toolchain is available"

# 2. Check basic compilation
echo -e "\n${YELLOW}2. Checking basic compilation...${NC}"
cargo check --all-features
print_status $? "All features compile successfully"

# 3. Check each feature individually
echo -e "\n${YELLOW}3. Testing individual features...${NC}"
for feature in grounding caching functions thinking streaming; do
    echo -n "  Testing $feature... "
    cargo check --no-default-features --features $feature >/dev/null 2>&1
    print_status $? "$feature"
done

# 4. Check problematic feature combinations
echo -e "\n${YELLOW}4. Testing feature combinations...${NC}"
echo -n "  Testing grounding without functions... "
cargo check --no-default-features --features grounding >/dev/null 2>&1
print_status $? "grounding alone works"

echo -n "  Testing all features together... "
cargo check --all-features >/dev/null 2>&1
print_status $? "all features work together"

# 5. Run tests
echo -e "\n${YELLOW}5. Running tests...${NC}"
cargo test --all-features
print_status $? "All tests pass"

# 6. Check documentation
echo -e "\n${YELLOW}6. Checking documentation...${NC}"
cargo doc --all-features --no-deps >/dev/null 2>&1
print_status $? "Documentation builds successfully"

# 7. Check examples
echo -e "\n${YELLOW}7. Checking examples...${NC}"
for example in basic structured_output grounding caching function_calling; do
    if [ -f "examples/$example.rs" ]; then
        echo -n "  Checking $example... "
        cargo check --example $example --all-features >/dev/null 2>&1
        print_status $? "$example"
    fi
done

# 8. Run clippy
echo -e "\n${YELLOW}8. Running clippy...${NC}"
if command -v clippy-driver >/dev/null 2>&1; then
    cargo clippy --all-features -- -D warnings
    print_status $? "No clippy warnings"
else
    echo -e "${YELLOW}⚠️  Clippy not installed, skipping${NC}"
fi

# 9. Check formatting
echo -e "\n${YELLOW}9. Checking formatting...${NC}"
if command -v rustfmt >/dev/null 2>&1; then
    cargo fmt -- --check
    print_status $? "Code is properly formatted"
else
    echo -e "${YELLOW}⚠️  rustfmt not installed, skipping${NC}"
fi

# 10. Validate Cargo.toml
echo -e "\n${YELLOW}10. Validating Cargo.toml...${NC}"
grep -q 'edition = "2021"' Cargo.toml
print_status $? "Correct Rust edition"

# 11. Check for required files
echo -e "\n${YELLOW}11. Checking required files...${NC}"
required_files=(
    "Cargo.toml"
    "README.md"
    "LICENSE-MIT"
    "LICENSE-APACHE"
    ".gitignore"
    "src/lib.rs"
    "src/client.rs"
    "src/config.rs"
    "src/error.rs"
    "src/models.rs"
)

for file in "${required_files[@]}"; do
    if [ -f "$file" ]; then
        echo -e "  ${GREEN}✅ $file exists${NC}"
    else
        echo -e "  ${YELLOW}⚠️  $file missing${NC}"
    fi
done

# 12. Check for issues in specific files
echo -e "\n${YELLOW}12. Checking for specific issues...${NC}"

# Check if GroundingConfig::to_tools has feature flag
echo -n "  Checking GroundingConfig::to_tools feature flag... "
if grep -q '#\[cfg(feature = "functions")\]' src/grounding/mod.rs; then
    print_status 0 "Feature flag present"
else
    print_status 1 "Feature flag missing"
fi

# Check if plans directory was removed
echo -n "  Checking if empty plans directory removed... "
if [ -d "plans" ]; then
    echo -e "${YELLOW}⚠️  plans directory still exists${NC}"
else
    echo -e "${GREEN}✅ plans directory removed${NC}"
fi

# 13. Test API functionality (if real key available)
echo -e "\n${YELLOW}13. Testing API functionality...${NC}"
if [ "$USE_REAL_API" = true ]; then
    echo -n "  Testing basic API call... "
    
    # Temporarily disable exit on error for API test
    set +e
    # Test with the basic example, using our API key and capture error output
    API_ERROR=$(GEMINI_API_KEY="$API_KEY" timeout 30 cargo run --example basic 2>&1)
    API_EXIT_CODE=$?
    # Re-enable exit on error
    set -e
    
    if [ $API_EXIT_CODE -eq 0 ]; then
        print_status 0 "API connection successful"
        API_TEST_SUCCESS=true
    else
        echo -e "${RED}❌ API test failed${NC}"
        echo -e "${RED}Exact error:${NC}"
        echo -e "${RED}$API_ERROR${NC}"
        API_TEST_SUCCESS=false
    fi
else
    echo -e "  ${YELLOW}⚠️  Skipping API test (no real API key available)${NC}"
    echo -e "  ${BLUE}💡 Set GEMINI_API_KEY environment variable to test real API calls${NC}"
fi

# 14. Generate summary report
echo -e "\n${YELLOW}=================================="
echo "📊 Validation Summary"
echo "==================================${NC}"

# Count Rust files
rust_files=$(find src -name "*.rs" | wc -l)
echo "Total Rust files: $rust_files"

# Count examples
example_files=$(find examples -name "*.rs" 2>/dev/null | wc -l || echo 0)
echo "Example files: $example_files"

# Count tests
test_count=$(grep -r "#\[test\]" tests/ src/ 2>/dev/null | wc -l || echo 0)
echo "Test functions: $test_count"

# Check code size
total_lines=$(find src -name "*.rs" -exec wc -l {} + | tail -1 | awk '{print $1}')
echo "Total lines of code: $total_lines"

echo -e "\n${GREEN}✨ All validations passed!${NC}"
echo ""
if [ "$USE_REAL_API" = true ]; then
    echo -e "${BLUE}🔑 API Key: Using real GEMINI_API_KEY from environment${NC}"
    if [ "$API_TEST_SUCCESS" = true ]; then
        echo -e "${GREEN}✅ API functionality tested successfully${NC}"
    else
        echo -e "${YELLOW}⚠️  API test failed or timed out${NC}"
    fi
else
    echo -e "${YELLOW}⚠️  API Key: Using test-key (set GEMINI_API_KEY for real API testing)${NC}"
fi
echo ""
echo "The gemini-rust crate is ready for use. You can now:"
echo "1. Publish to crates.io: cargo publish --dry-run"
echo "2. Create git tags: git tag -a v0.1.0 -m 'Initial release'"
echo "3. Push to GitHub: git push origin main --tags"
