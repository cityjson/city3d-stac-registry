# CityJSON-STAC Justfile

# Default recipe
default:
    @just --list

# Generate STAC types from JSON schemas
gen:
    cargo build
    @echo "Generated types in target/debug/build/cityjson_stac-*/out/stac_types.rs"

# Clean and regenerate
regen: clean-gen gen

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
ci: fmt-check clippy test

# run dev container
devcon:
    devcontainer up --workspace-folder .
    devcontainer exec --workspace-folder . bash

devcon-build:
    devcontainer build --workspace-folder . --no-cache
    just devcon