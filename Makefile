# Makefile for cityjson-stac
# Run `make help` to see available commands

.PHONY: all build release test lint fmt check clean doc install help setup ci

# Default target
all: check

# ============================================================================
# Development Commands
# ============================================================================

## Build the project in debug mode
build:
	cargo build

## Build the project in release mode
release:
	cargo build --release

## Run all tests
test:
	cargo test --all-features

## Run tests with output
test-verbose:
	cargo test --all-features -- --nocapture

## Run clippy linter
lint:
	cargo clippy --all-targets --all-features -- -D warnings

## Check code formatting
fmt-check:
	cargo fmt --all -- --check

## Format code
fmt:
	cargo fmt --all

## Run cargo check (fast compilation check)
check:
	cargo check --all-targets --all-features

## Run all CI checks (format, lint, test)
ci: fmt-check lint test
	@echo "✅ All CI checks passed!"

## Run all checks and fix what can be fixed
fix: fmt
	cargo clippy --fix --all-targets --all-features --allow-dirty

# ============================================================================
# Documentation
# ============================================================================

## Generate documentation
doc:
	cargo doc --no-deps --all-features

## Generate and open documentation
doc-open:
	cargo doc --no-deps --all-features --open

# ============================================================================
# Utilities
# ============================================================================

## Clean build artifacts
clean:
	cargo clean

## Install the binary locally
install:
	cargo install --path .

## Setup development environment (git hooks)
setup:
	chmod +x scripts/setup-hooks.sh
	./scripts/setup-hooks.sh

## Run security audit
audit:
	cargo audit

## Update dependencies
update:
	cargo update

## Show outdated dependencies
outdated:
	cargo outdated

# ============================================================================
# Run Commands
# ============================================================================

## Run with debug logging
run-debug:
	RUST_LOG=debug cargo run -- $(ARGS)

## Run in release mode
run-release:
	cargo run --release -- $(ARGS)

# ============================================================================
# Examples
# ============================================================================

## Generate example STAC item from test data
example-item:
	cargo run -- item tests/data/delft.city.json -o target/example_item.json --pretty
	@echo "Generated: target/example_item.json"

## Generate example STAC collection from test data
example-collection:
	cargo run -- collection tests/data -o target/example_collection --pretty
	@echo "Generated: target/example_collection/"

# ============================================================================
# Help
# ============================================================================

## Show this help message
help:
	@echo "cityjson-stac Development Commands"
	@echo "==================================="
	@echo ""
	@echo "Usage: make [target]"
	@echo ""
	@echo "Development:"
	@echo "  build          Build in debug mode"
	@echo "  release        Build in release mode"
	@echo "  test           Run all tests"
	@echo "  test-verbose   Run tests with output"
	@echo "  lint           Run clippy linter"
	@echo "  fmt            Format code"
	@echo "  fmt-check      Check code formatting"
	@echo "  check          Fast compilation check"
	@echo "  ci             Run all CI checks (format, lint, test)"
	@echo "  fix            Auto-fix formatting and lint issues"
	@echo ""
	@echo "Documentation:"
	@echo "  doc            Generate documentation"
	@echo "  doc-open       Generate and open documentation"
	@echo ""
	@echo "Utilities:"
	@echo "  clean          Clean build artifacts"
	@echo "  install        Install binary locally"
	@echo "  setup          Setup git hooks"
	@echo "  audit          Run security audit"
	@echo "  update         Update dependencies"
	@echo "  outdated       Show outdated dependencies"
	@echo ""
	@echo "Run:"
	@echo "  run-debug      Run with debug logging (use ARGS=...)"
	@echo "  run-release    Run in release mode (use ARGS=...)"
	@echo ""
	@echo "Examples:"
	@echo "  example-item       Generate example STAC item"
	@echo "  example-collection Generate example STAC collection"
	@echo ""
	@echo "Usage examples:"
	@echo "  make ci                              # Run all checks"
	@echo "  make test                            # Run tests"
	@echo "  make fix                             # Fix formatting and lint issues"
	@echo "  make run-debug ARGS='item file.json -o out.json'"
