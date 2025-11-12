# Cargo.toml Structure

## Project Configuration

```toml
[package]
name = "cityjson-stac"
version = "0.1.0"
edition = "2021"
authors = ["Your Name <your.email@example.com>"]
description = "Generate STAC metadata for CityJSON datasets"
license = "MIT OR Apache-2.0"
repository = "https://github.com/cityjson/cityjson-stac"
readme = "README.md"
keywords = ["stac", "cityjson", "3d", "gis", "geospatial"]
categories = ["command-line-utilities", "science::geo"]

[dependencies]
# CLI
clap = { version = "4.5", features = ["derive", "cargo"] }

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Error handling
anyhow = "1.0"
thiserror = "1.0"

# File I/O and directory traversal
walkdir = "2.4"

# Date/time handling
chrono = { version = "0.4", features = ["serde"] }

# Geometry
geojson = { version = "0.24", features = ["geo-types"] }
geo-types = "0.7"

# Logging
log = "0.4"
env_logger = "0.11"

# For CityJSON format (streaming JSON parsing)
# We'll use serde_json but may need ijson-like for large files
# Consider: json-stream-rs or similar for .jsonl

# For FlatCityBuf format
flatbuffers = "24.3"

# URL handling for STAC links
url = "2.5"

# Optional: for parallel processing
rayon = { version = "1.10", optional = true }

[dev-dependencies]
tempfile = "3.10"
assert_cmd = "2.0"
predicates = "3.1"
pretty_assertions = "1.4"

[features]
default = []
parallel = ["rayon"]

[[bin]]
name = "cityjson-stac"
path = "src/main.rs"

[lib]
name = "cityjson_stac"
path = "src/lib.rs"

[profile.release]
lto = true
codegen-units = 1
opt-level = 3
strip = true
```

## Workspace Structure (Future Expansion)

If we split into multiple crates:

```toml
[workspace]
members = [
    "cityjson-stac-cli",
    "cityjson-stac-core",
    "cityjson-stac-reader",
]
resolver = "2"

[workspace.package]
version = "0.1.0"
edition = "2021"
license = "MIT OR Apache-2.0"

[workspace.dependencies]
# Shared dependencies
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
anyhow = "1.0"
thiserror = "1.0"
```

## Dependency Rationale

### Core Dependencies

| Dependency | Version | Purpose | Why This Choice |
|------------|---------|---------|-----------------|
| `clap` | 4.5 | CLI parsing | Industry standard, excellent derive macros, great UX |
| `serde` + `serde_json` | 1.0 | JSON serialization | De facto standard for Rust, zero-cost abstractions |
| `anyhow` | 1.0 | Error propagation | Ergonomic error handling for application code |
| `thiserror` | 1.0 | Error types | Clean derive macros for custom error types |
| `walkdir` | 2.4 | Directory traversal | Battle-tested, handles symlinks, cross-platform |
| `chrono` | 0.4 | Date/time | Standard datetime library with STAC-compatible RFC3339 |
| `geojson` | 0.24 | GeoJSON handling | Native GeoJSON support for STAC geometries |
| `flatbuffers` | 24.3 | FlatBuffers parsing | Required for FlatCityBuf format |

### Optional Dependencies

| Dependency | Purpose | When to Enable |
|------------|---------|----------------|
| `rayon` | Parallel processing | Large directories (1000+ files) |

### Development Dependencies

| Dependency | Purpose |
|------------|---------|
| `tempfile` | Temporary test files |
| `assert_cmd` | CLI testing |
| `predicates` | Test assertions |
| `pretty_assertions` | Better test failure messages |

## Build Configuration

### Release Profile

Optimized for distribution:
- **LTO (Link Time Optimization)**: Enabled for smaller binary size and better performance
- **Codegen Units**: 1 for maximum optimization (slower compile, faster binary)
- **Opt Level**: 3 for aggressive optimizations
- **Strip**: Remove debug symbols for smaller binary

### Development Profile (default)

Fast compilation for development:
- Default settings
- Debug symbols included
- No LTO

## Platform Support

### Target Platforms

- **Linux**: Primary development platform
- **macOS**: Full support
- **Windows**: Full support

### Cross-compilation

```bash
# Build for Linux
cargo build --release --target x86_64-unknown-linux-gnu

# Build for macOS
cargo build --release --target x86_64-apple-darwin

# Build for Windows
cargo build --release --target x86_64-pc-windows-msvc
```

## Installation Methods

### From crates.io

```bash
cargo install cityjson-stac
```

### From source

```bash
git clone https://github.com/cityjson/cityjson-stac.git
cd cityjson-stac
cargo install --path .
```

### From binary release

Download pre-built binaries from GitHub Releases.

## Testing

```bash
# Run all tests
cargo test

# Run with logging
RUST_LOG=debug cargo test

# Run specific test
cargo test test_cityjson_reader

# Run integration tests
cargo test --test integration

# Run with coverage (requires cargo-tarpaulin)
cargo tarpaulin --out Html
```

## Benchmarking

```bash
# Add to dev-dependencies for benchmarking
[dev-dependencies]
criterion = "0.5"

# Run benchmarks
cargo bench
```

## Documentation

```bash
# Generate and open docs
cargo doc --open --no-deps

# Check for broken links
cargo deadlinks
```

## CI/CD Configuration

### GitHub Actions

```yaml
# .github/workflows/ci.yml
name: CI

on: [push, pull_request]

jobs:
  test:
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
        rust: [stable, beta]
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@master
        with:
          toolchain: ${{ matrix.rust }}
      - run: cargo test --all-features
      - run: cargo build --release

  lint:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy
      - run: cargo fmt -- --check
      - run: cargo clippy -- -D warnings
```

## Feature Flags

### `parallel`

Enable parallel processing for directory traversal:

```bash
cargo build --features parallel
```

When to use:
- Processing 1000+ files
- Multi-core systems
- Performance-critical applications

Overhead:
- Slightly larger binary
- Thread pool initialization cost

## Minimum Supported Rust Version (MSRV)

**MSRV: 1.70.0**

Rationale:
- Stable feature set
- Good ecosystem compatibility
- Not too old for modern dependencies

## Size Optimization

For minimal binary size:

```toml
[profile.release]
opt-level = "z"  # Optimize for size
lto = true
codegen-units = 1
panic = "abort"
strip = true
```

Expected sizes:
- Default release: ~8-10 MB
- Size-optimized: ~4-6 MB
- With upx compression: ~2-3 MB

## Development Tools

Recommended tools for development:

```bash
# Install dev tools
cargo install cargo-watch      # Auto-recompile on changes
cargo install cargo-edit        # Manage dependencies
cargo install cargo-outdated    # Check for outdated deps
cargo install cargo-audit       # Security auditing
cargo install cargo-bloat       # Binary size analysis
cargo install cargo-tarpaulin   # Code coverage
```

## Pre-commit Hooks

```bash
# .git/hooks/pre-commit
#!/bin/bash
cargo fmt -- --check
cargo clippy -- -D warnings
cargo test
```
