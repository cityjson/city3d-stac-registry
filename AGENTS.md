# CityJSON-STAC Agent Guidelines

This document serves as the master reference for AI coding agents working on this project.

## Project Overview

**cityjson-stac** is a Rust-based CLI tool that generates [STAC (SpatioTemporal Asset Catalog)](https://stacspec.org/) metadata from 3D city model datasets. It bridges the gap between STAC's geospatial metadata standard and CityJSON formats.

### Core Problem

STAC is widely adopted for geospatial data but lacks native support for 3D city models. This tool automatically generates STAC Items and Collections from CityJSON datasets with a custom extension for 3D-specific metadata.

### Supported Input Formats

| Format                | Extension | Library                                                           | Status |
| --------------------- | --------- | ----------------------------------------------------------------- | ------ |
| CityJSON              | `.json`   | `serde_json`                                                      | ✅     |
| CityJSONTextSequences | `.jsonl`  | `serde_json` (streaming)                                          | ✅     |
| FlatCityBuf           | `.fcb`    | [flatcitybuf](https://github.com/cityjson/flatcitybuf) Rust crate | 🚧     |

## High-Level Architecture

```
┌─────────────────────────────────────────────────────────┐
│                     CLI Interface                        │
│               (clap command routing)                     │
└─────────────────┬───────────────────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────────────────┐
│              Reader Factory                              │
│        (file extension → reader selection)              │
└─────────────────┬───────────────────────────────────────┘
                  │
        ┌─────────┴──────────┬──────────────┐
        ▼                    ▼              ▼
┌──────────────┐   ┌──────────────┐   ┌──────────────┐
│  CityJSON    │   │ CityJSONSeq  │   │ FlatCityBuf  │
│   Reader     │   │   Reader     │   │   Reader     │
└──────────────┘   └──────────────┘   └──────────────┘
        │                    │              │
        └─────────┬──────────┴──────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────────────────┐
│         CityModelMetadataReader Trait                   │
│                                                          │
│   - bbox() → BBox3D                                     │
│   - crs() → CRS                                         │
│   - lods() → Vec<String>                                │
│   - city_object_types() → Vec<String>                   │
│   - city_object_count() → usize                         │
│   - attributes() → Vec<AttributeDefinition>             │
└─────────────────┬───────────────────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────────────────┐
│               STAC Generation                           │
│     (StacItemBuilder / StacCollectionBuilder)           │
└─────────────────┬───────────────────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────────────────┐
│              JSON Output                                │
│   (item.json / collection.json + items/)                │
└─────────────────────────────────────────────────────────┘
```

## Project Structure

```
cityjson-stac/
├── src/
│   ├── main.rs              # CLI entry point
│   ├── lib.rs               # Library exports
│   ├── error.rs             # Error types
│   ├── cli/
│   │   └── mod.rs           # CLI commands (item, collection)
│   ├── reader/
│   │   ├── mod.rs           # Reader trait & factory
│   │   └── cityjson.rs      # CityJSON reader implementation
│   ├── metadata/
│   │   ├── mod.rs           # Metadata exports
│   │   ├── bbox.rs          # 3D bounding box
│   │   ├── crs.rs           # Coordinate reference system
│   │   ├── transform.rs     # Coordinate transform
│   │   └── attributes.rs    # Attribute definitions
│   ├── stac/
│   │   ├── mod.rs           # STAC exports
│   │   ├── models.rs        # STAC data models
│   │   ├── item.rs          # STAC Item builder
│   │   └── collection.rs    # STAC Collection builder
│   └── traversal/
│       └── mod.rs           # Directory scanning
├── stac-cityjson-extension/  # Official STAC 3D City Models Extension (submodule)
│   └── json-schema/
│       └── schema.json     # Extension JSON Schema
└── docs/
    └── examples/            # Example STAC outputs
```

## Important References

### Specifications

- [STAC Specification v1.1.0](https://stacspec.org/) - Core STAC standard
- [CityJSON Specification](https://www.cityjson.org/specs/) - CityJSON format spec
- [STAC Extensions Registry](https://stac-extensions.github.io/) - Extension examples

### Related Libraries

- [FlatCityBuf](https://github.com/cityjson/flatcitybuf) - For `.fcb` format support, use the Rust crate under `src/rust`
- [CityJSON Sequences](https://github.com/cityjson/cjseq) - For `.jsonl` format reference

### Project Documentation

- [STAC_EXTENSION.md](./STAC_EXTENSION.md) - STAC 3D City Models Extension specification
- [DESIGN_DOC.md](./DESIGN_DOC.md) - Detailed technical architecture

## Coding Guidelines

### Pre-Commit Checklist

Before committing any code, ensure:

1. **Format**: `cargo fmt`
2. **Lint**: `cargo clippy -- -D warnings` (no warnings allowed)
3. **Test**: `cargo test --lib` (all unit tests pass)

The git hooks enforce this automatically. To bypass in emergencies:

```bash
git commit --no-verify -m "emergency fix"
```

### Design Philosophy

**Interface Programming**: All readers implement `CityModelMetadataReader` trait

- Factory pattern (`get_reader()`) returns appropriate reader based on file extension
- Caller doesn't need to know internal implementation (local vs remote, format specifics)
- Trait abstraction enables polymorphism and extensibility

**Streaming-First Approach**:

- CityJSONSeq uses streaming (`BufReader::lines()`) for memory efficiency
- CityJSON loads entirely (typically small files)
- All readers use lazy loading via `RwLock` for interior mutability

**Module Dependencies**: Reader → Metadata (not vice versa)

- `CityModelMetadataReader` trait in `src/reader/mod.rs`
- Metadata types (`BBox3D`, `CRS`, `Transform`, `AttributeDefinition`) in `src/metadata/`
- STAC builders consume metadata from readers

**Never Use**:

- `#[allow(dead_code)]` or `#[allow(unused)]` as workarounds
- Real data in tests (use mocked/fabricated data)

### Rust Conventions

1. **Error Handling**: Use `thiserror` for library errors, `anyhow` for application-level errors
2. **Serialization**: All STAC structures must derive `Serialize`/`Deserialize` with serde
3. **Traits**: Format readers implement `CityModelMetadataReader` trait for polymorphism
4. **Builders**: STAC Items/Collections use builder pattern for construction

### Design Patterns

```rust
// Factory pattern for reader selection
pub fn get_reader(path: &Path) -> Result<Box<dyn CityModelMetadataReader>>

// Trait-based abstraction for formats
pub trait CityModelMetadataReader: Send + Sync {
    fn bbox(&self) -> Result<BBox3D>;
    fn crs(&self) -> Result<CRS>;
    fn lods(&self) -> Result<Vec<String>>;
    fn city_object_types(&self) -> Result<Vec<String>>;
    fn city_object_count(&self) -> Result<usize>;
    fn encoding(&self) -> &'static str;
    // ...
}

// Builder pattern for STAC generation
StacItemBuilder::new("my-item")
    .bbox(reader.bbox()?)
    .cityjson_metadata(&reader)?
    .build()?
```

### CLI Commands

| Command             | Description                                   |
| ------------------- | --------------------------------------------- |
| `item`              | Generate STAC Item from single file           |
| `collection`        | Generate STAC Collection from directory       |
| `update-collection` | Aggregate STAC Collection from existing items |

The `update-collection` command (alias: `aggregate`) is useful for Object Storage scenarios where STAC items are generated individually and then aggregated into a collection.

### Testing

- Unit tests in each module
- Integration tests in `tests/`
- Test fixtures in `tests/fixtures/`
- Run with: `cargo test`

### Adding a New Format Reader

1. Create new file in `src/reader/` (e.g., `fcb.rs`)
2. Implement `CityModelMetadataReader` trait
3. Register in factory at `src/reader/mod.rs`
4. Add file extension matching in `get_reader()`

## STAC Extension Properties (city3d: prefix)

The tool uses the [STAC 3D City Models Extension](https://cityjson.github.io/stac-city3d/v0.1.0/schema.json).

| Property                   | Type          | Description               |
| -------------------------- | ------------- | ------------------------- |
| `city3d:encoding`          | string        | Format name (required)    |
| `city3d:version`           | string        | Specification version     |
| `city3d:encoding_version`  | string        | Encoding-specific version |
| `city3d:city_objects`      | integer/stats | Object count              |
| `city3d:lods`              | array[string] | Levels of detail          |
| `city3d:co_types`          | array[string] | City object types         |
| `city3d:attributes`        | array[object] | Attribute schema          |
| `city3d:semantic_surfaces` | boolean       | Has semantic surfaces     |
| `city3d:textures`          | boolean       | Has texture information   |
| `city3d:materials`         | boolean       | Has material information  |

The extension also references:

- [Projection v2.0.0](https://stac-extensions.github.io/projection/v2.0.0/schema.json)
- [File v2.1.0](https://stac-extensions.github.io/file/v2.1.0/schema.json)
- [Stats v0.2.0](https://stac-extensions.github.io/stats/v0.2.0/schema.json)

See [STAC_EXTENSION.md](./STAC_EXTENSION.md) for full specification.

## Development Setup

### Prerequisites

- Rust 1.70+ (install via [rustup](https://rustup.rs))
- Git

### First-Time Setup

After cloning the repository, set up git hooks:

```bash
# Make setup script executable and run it
chmod +x scripts/setup-hooks.sh
./scripts/setup-hooks.sh
```

This configures **pre-commit hooks** that automatically run before each commit:

1. **Format check** (`cargo fmt --check`) - Ensures consistent code formatting
2. **Lint check** (`cargo clippy`) - Catches common mistakes and enforces best practices
3. **Quick tests** (`cargo test --lib`) - Runs unit tests to catch regressions

If any check fails, the commit is blocked until the issues are fixed.

### Manual Pre-commit Checks

```bash
# Run the same checks manually
cargo fmt --check          # Check formatting
cargo clippy -- -D warnings # Check linting
cargo test --lib            # Run unit tests

# Fix formatting automatically
cargo fmt

# Run all checks at once
cargo fmt && cargo clippy -- -D warnings && cargo test
```

### Bypassing Hooks (Emergency Only)

```bash
git commit --no-verify -m "emergency fix"
```

### Disabling Hooks

```bash
git config --unset core.hooksPath
```

## CI/CD

### GitHub Actions Workflows

| Workflow      | Trigger             | Description                                               |
| ------------- | ------------------- | --------------------------------------------------------- |
| `ci.yml`      | Push to main, PRs   | Runs check, format, clippy, test, docs, security audit    |
| `release.yml` | Version tags (`v*`) | Builds binaries for all platforms, creates GitHub Release |

### CI Jobs

| Job      | Command                 | Purpose                                       |
| -------- | ----------------------- | --------------------------------------------- |
| Check    | `cargo check`           | Fast compilation check                        |
| Format   | `cargo fmt --check`     | Verify code formatting                        |
| Clippy   | `cargo clippy`          | Linting with warnings as errors               |
| Test     | `cargo test`            | Run all tests                                 |
| Build    | `cargo build --release` | Cross-platform builds (Linux, macOS, Windows) |
| Docs     | `cargo doc`             | Ensure documentation builds                   |
| Security | `cargo audit`           | Check for vulnerable dependencies             |

### Creating a Release

```bash
# Create and push a version tag
git tag v0.1.0
git push origin v0.1.0

# This triggers the release workflow which:
# - Builds binaries for Linux, macOS, Windows (AMD64 + ARM64)
# - Creates a GitHub Release with all artifacts and checksums
```

### Dependabot

Automated dependency updates run weekly (Mondays) for:

- Cargo dependencies
- GitHub Actions

## Quick Reference

```bash
# Build
cargo build --release

# Test
cargo test

# Check everything (same as CI)
cargo fmt --check && cargo clippy -- -D warnings && cargo test

# Generate STAC Item (relative href)
cityjson-stac item building.json -o building_item.json

# Generate STAC Item with absolute URL
cityjson-stac item building.json --base-url https://data.example.com/files -o building_item.json

# Generate STAC Collection
cityjson-stac collection ./data/ -o ./stac_output

# Generate STAC Collection with absolute URLs
cityjson-stac collection ./data/ -o ./stac_output --base-url https://data.example.com/files

# Aggregate STAC Collection from existing items (for Object Storage workflows)
cityjson-stac update-collection item1.json item2.json item3.json -o collection.json

# Aggregate with absolute item URLs
cityjson-stac update-collection items/*.json --items-base-url https://example.com/stac/items -o collection.json

# Debug logging
RUST_LOG=debug cargo run -- item file.json -o output.json
```

### CLI Options

| Option             | Commands          | Description                                                                                                 |
| ------------------ | ----------------- | ----------------------------------------------------------------------------------------------------------- |
| `--base-url`       | item, collection  | Base URL for asset hrefs. Without it, hrefs are relative (filename only). With it, hrefs are absolute URLs. |
| `--items-base-url` | update-collection | Base URL for item links in the collection. Without it, links are relative to the collection.                |
| `--dry-run`        | all              | Validate config and inputs without generating output. Exits: 0=valid, 1=config error, 2=path error, 3=URL error |

### Dry Run Mode

Validate configuration and inputs before processing:

```bash
# Validate collection config
cityjson-stac collection --config config.yaml --dry-run

# Validate item input (local or remote)
cityjson-stac item https://example.com/data.json --dry-run

# Validate update-collection inputs
cityjson-stac update-collection items/*.json --dry-run

# Validate catalog configuration
cityjson-stac catalog --config catalog-config.yaml --dry-run
```

**Exit codes:**
- `0` - All validations passed
- `1` - Config file error (syntax/semantic)
- `2` - Missing input paths
- `3` - Remote URL inaccessible

### Filename Collision Handling

When processing a collection with files that have the same stem but different extensions (e.g., `delft.city.json` and `delft.city.jsonl`), item IDs and filenames get format-specific suffixes to avoid collisions:

| Format                 | Suffix   | Example            |
| ---------------------- | -------- | ------------------ |
| CityJSON (`.json`)     | `_cj`    | `delft.city_cj`    |
| CityJSONSeq (`.jsonl`) | `_cjseq` | `delft.city_cjseq` |
| FlatCityBuf (`.fcb`)   | `_fcb`   | `delft.city_fcb`   |
