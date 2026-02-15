# City 3D STAC Justfile

# Default recipe
default:
    @just --list

# Generate STAC types from JSON schemas
#
# Note: STAC types are manually maintained in src/stac/models.rs
# These types are derived from STAC v1.0.0 JSON schemas and match
# the official STAC specification structure.
#
# To modify STAC types:
# 1. Edit src/stac/models.rs
# 2. Run `cargo build` to recompile with changes
# 3. Run tests to verify changes

# ============================================================================
# Development
# ============================================================================

# Build the project
build:
    cargo build

# Build in release mode
release:
    cargo build --release

# Clean generated files
clean:
    cargo clean

# Alias for clean (legacy)
clean-gen: clean

# Clean and rebuild
regen: clean build

# install the binary locally
install:
    cargo install --path .

# Setup development environment (git hooks)
setup:
    chmod +x scripts/setup-hooks.sh
    ./scripts/setup-hooks.sh

# ============================================================================
# Checks & Testing
# ============================================================================

# Run tests (using nextest)
test:
    cargo nextest run --all-features

# Run tests with standard cargo test and output (verbose)
test-verbose:
    cargo test --all-features -- --nocapture

# Check formatting
fmt-check:
    cargo fmt --check

# Format code
fmt:
    cargo fmt

# Run clippy (check only, fail on warnings)
lint-check:
    cargo clippy --all-targets --all-features -- -D warnings

# Run clippy and fix issues
lint:
    cargo clippy --fix --allow-dirty --all-features
    cargo check --all-features

# Fast compilation check
check:
    cargo check --all-targets --all-features

# Run security audit
audit:
    cargo audit

# Show outdated dependencies
outdated:
    cargo outdated

# Update dependencies
update:
    cargo update

# Full CI check
ci: fmt-check lint-check test build

# Pre-commit task
pre-commit: fmt lint test build

# ============================================================================
# Documentation
# ============================================================================

# Generate documentation
doc:
    cargo doc --no-deps --all-features

# Generate and open documentation
doc-open:
    cargo doc --no-deps --all-features --open

# ============================================================================
# Run
# ============================================================================

# Run with debug logging
run-debug +args='':
    RUST_LOG=debug cargo run -- {{args}}

# Run in release mode
run-release +args='':
    cargo run --release -- {{args}}

# ============================================================================
# Examples
# ============================================================================

# Generate example STAC item from test data
example-item:
    cargo run -- item tests/data/delft.city.json -o target/example_item.json --pretty
    @echo "Generated: target/example_item.json"

# Generate example STAC collection from test data
example-collection:
    cargo run -- collection tests/data -o target/example_collection --pretty
    @echo "Generated: target/example_collection/"

# ============================================================================
# Dev Container
# ============================================================================

# run dev container
devcon:
    devcontainer up --workspace-folder .
    devcontainer exec --workspace-folder . bash

# rebuild dev container
devcon-build:
    devcontainer build --workspace-folder . --no-cache
    just devcon
