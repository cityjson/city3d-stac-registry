# City3D STAC Registry Justfile

tool := "city3d-stac-tool"
cargo_flags := "--manifest-path " + tool + "/Cargo.toml"

# Default: list available recipes
default:
    @just --list

# ============================================================================
# Build & Install
# ============================================================================

# Build the tool (debug)
build:
    cargo build {{ cargo_flags }}

# Build the tool (release)
release:
    cargo build --release {{ cargo_flags }}

# Install the binary locally
install:
    cargo install --path {{ tool }}

# ============================================================================
# Validation (dry-run)
# ============================================================================

# Validate a single collection (e.g. just validate-collection amsterdam-config)
validate-collection name:
    cargo run {{ cargo_flags }} -- collection --config collections/{{ name }}.yaml --dry-run

# Validate the full catalog
validate-catalog:
    cargo run {{ cargo_flags }} -- catalog --config catalog/catalog-config.yaml --dry-run

# Validate all collections one by one
validate-all:
    #!/usr/bin/env bash
    set -euo pipefail
    for f in collections/*.yaml; do
        name=$(basename "$f" .yaml)
        echo "==> Validating $name"
        cargo run {{ cargo_flags }} -- collection --config "$f" --dry-run
    done

# ============================================================================
# Generation
# ============================================================================

# Generate STAC for a single collection
generate-collection name output="output":
    cargo run {{ cargo_flags }} -- collection --config collections/{{ name }}.yaml -o {{ output }}/{{ name }} --pretty

# Generate the full catalog
generate-catalog output="output":
    cargo run {{ cargo_flags }} -- catalog --config catalog/catalog-config.yaml -o {{ output }} --pretty --geoparquet --overwrite

# ============================================================================
# Submodule
# ============================================================================

# Initialize / update the tool submodule
submodule-update:
    git submodule update --init --recursive

# Run tests in the tool submodule
tool-test:
    cargo test {{ cargo_flags }} --all-features

# Run clippy on the tool
tool-lint:
    cargo clippy {{ cargo_flags }} --all-targets --all-features -- -D warnings
