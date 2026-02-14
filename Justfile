# CityJSON-STAC Justfile

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

# Build the project
build:
    cargo build

# Clean and rebuild
regen: clean-gen build

# Clean generated files
clean-gen:
    cargo clean

# Run tests
test:
    cargo test

# Check formatting
fmt-check:
    cargo fmt --check

# Format code
fmt:
    cargo fmt

# Run clippy
clippy:
    cargo clippy -- -D warnings

# Full CI check
ci: fmt clippy test

# run dev container
devcon:
    devcontainer up --workspace-folder .
    devcontainer exec --workspace-folder . bash

# rebuild dev container
devcon-build:
    devcontainer build --workspace-folder . --no-cache
    just devcon
