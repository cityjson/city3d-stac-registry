# CityJSON-STAC Refactoring Tasks

## Overview

This document tracks the progress of refactoring the cityjson-stac codebase to use the `cjseq` crate's typed API where appropriate, implement streaming processing for CityJSONSeq, and document the design philosophy.

---

## Task 1: Create `tasks.md` and Update Documentation

### Status: ✅ Complete

#### 1.1 Create `tasks.md` file

- [x] Create tasks.md to track progress

#### 1.2 Update `DESIGN_DOC.md`

- [x] Add **Design Philosophy Section** at top after Architecture Details
  - Interface programming with `CityModelMetadataReader` trait
  - Factory pattern for reader selection
  - Streaming-first approach (especially for CityJSONSeq)
  - Abstraction of data source (local, HTTP, object storage)
- [x] Add **Development Standards Section**
  - Test-Driven Development (TDD) requirement
  - Always run `cargo clippy` and `cargo fmt` before commits
  - NEVER use `#[allow(dead_code)]` or `#[allow(unused)]` as workarounds
  - Return mocked data in tests, not real data
- [x] Add **Test Data Section**
  - Document `tests/data/` directory
  - List available test files (delft.city.json, delft.city.jsonl, railway files, all.fcb)

#### 1.3 Update `CLAUDE.md`

- [x] Reinforce design philosophy
- [x] Add pre-commit checklist

---

## Task 2 & 3: Use cjseq for Both CityJSON and CityJSONSeq Readers

### Status: 🚧 In Progress

### Implementation Approach

**Decision:** Use `cjseq` crate's typed API for **both** `CityJSONReader` and `CityJSONSeqReader`.

The cjseq crate provides strongly-typed Rust structs for CityJSON data:
- `cjseq::CityJSON` - For CityJSON files
- `cjseq::CityJSONFeature` - For CityJSONSeq feature lines
- `cjseq::CityObject`, `cjseq::Geometry`, `cjseq::Transform` - For accessing data

Both readers should:
1. Parse data using cjseq's `from_str()` methods
2. Access typed fields directly instead of JSON pointer navigation
3. Handle cases where files don't conform to cjseq's expected schema

### Implementation Note

The cjseq crate has a strict schema. Files that don't conform (e.g., missing required fields, incompatible types) should be rejected with clear error messages. This is preferable to silently accepting invalid data.

#### 2.1 CityJSONReader Changes

**File:** `src/reader/cityjson.rs`

Replace `serde_json::Value` with `cjseq::CityJSON`:
```rust
pub struct CityJSONReader {
    file_path: PathBuf,
    data: RwLock<Option<cjseq::CityJSON>>,  // Changed from Value
}
```

Key implementation points:
- Use `cjseq::CityJSON::from_str()` for parsing
- Access fields directly: `data.version`, `data.transform`, `data.metadata`
- Iterate `city_objects` for types, LODs, attributes
- Handle optional `transform` field gracefully

#### 2.2 CityJSONSeqReader Changes

**File:** `src/reader/cjseq.rs`

Replace `serde_json::Value` with `cjseq` types:
```rust
pub struct CityJSONSeqReader {
    file_path: PathBuf,
    metadata_header: cjseq::CityJSON,  // Changed from Value
    aggregated: RwLock<Option<AggregatedMetadata>>,
}
```

Key implementation points:
- Parse header line with `cjseq::CityJSON::from_str()`
- Parse feature lines with `cjseq::CityJSONFeature::from_str()`
- Use `HashSet` instead of `BTreeSet` for better performance (sorting not required)
- Stream features and aggregate statistics incrementally

#### 2.3 Enhancements made

- [x] Fixed CRS extraction to handle URL string format (`"https://www.opengis.net/def/crs/EPSG/0/7415"`)
- [x] Fixed `file_path()` to return `&Path` instead of `Result<PathBuf>`
- [x] Removed `AttributeType::Integer` (doesn't exist, use `Number` instead)
- [ ] Replace `serde_json::Value` with `cjseq::CityJSON` in CityJSONReader
- [ ] Replace `serde_json::Value` with `cjseq` types in CityJSONSeqReader
- [ ] Replace `BTreeSet` with `HashSet` in CityJSONSeqReader

#### 2.4 Verification

- [x] `cargo test` passes (64 tests baseline)
- [x] `cargo clippy -- -D warnings` passes
- [x] `cargo fmt --check` passes
- [ ] All tests pass with cjseq implementation

---

---

## Task 3: CityJSONSeqReader Streaming Implementation

### Status: 🚧 In Progress

See Task 2 above - CityJSONSeqReader is part of the combined refactoring effort to use cjseq for both readers.

### Streaming Design

CityJSONSeq is designed for streaming. The reader should:
1. Read first line as `cjseq::CityJSON` header (metadata only)
2. Stream remaining lines using `BufReader::lines()`
3. Parse each line as `cjseq::CityJSONFeature`
4. Aggregate statistics incrementally (LODs, types, attributes, etc.)
5. Discard features after extracting metadata (memory-efficient)

```rust
pub struct CityJSONSeqReader {
    file_path: PathBuf,
    metadata_header: cjseq::CityJSON,  // First line (metadata only)
    aggregated: RwLock<Option<AggregatedMetadata>>,
}

pub struct AggregatedMetadata {
    bbox: Option<BBox3D>,
    lods: HashSet<String>,  // Use HashSet, not BTreeSet (no sorting needed)
    city_object_types: HashSet<String>,
    city_object_count: usize,
    attributes: HashMap<String, AttributeType>,
    has_semantic_surfaces: bool,
    has_textures: bool,
    has_materials: bool,
}
```

---

## Task 4: Remote Data Access

### Status: ⏳ Deferred

The note says "Work on 1, 2 and 3 in order", so this is for later.

---

## Verification Checklist

After implementation is complete:

- [ ] Run `cargo test --lib` - All unit tests pass
- [ ] Run `cargo clippy -- -D warnings` - No warnings
- [ ] Run `cargo fmt --check` - Properly formatted
- [ ] Test with real data in `tests/data/`:
  - [ ] `delft.city.json` - Small CityJSON file
  - [ ] `delft.city.jsonl` - CityJSONSeq file
  - [ ] `railway.city.json` / `railway.city.jsonl` - Larger files

### Current Status

- [x] Documentation updates complete (Task 1)
- [ ] CityJSONReader using cjseq (Task 2)
- [ ] CityJSONSeqReader using cjseq (Task 3)

---

## End-to-End Test

```bash
# Build
cargo build --release

# Test item generation
cargo run -- item tests/data/delft.city.json -o /tmp/test_item.json

# Test collection generation
cargo run -- collection tests/data/ -o /tmp/test_collection/

# Verify JSON output is valid
cat /tmp/test_item.json | jq .
```

---

## Notes

- Always run `cargo fmt` before committing
- Always run `cargo clippy -- -D warnings` before committing
- Use `BTreeSet` for automatic sorting of collections
- Return `Result<T>` from all fallible operations
- Use `RwLock` for thread-safe lazy loading
- Document public APIs with rustdoc comments
