# CityJSON-STAC Implementation Progress

## Task Overview

Two main improvements to implement:

1. **Replace `serde_json::Value` with `cjseq` library** - Use the structured in-memory representation from https://github.com/cityjson/cjseq
2. **Implement FlatCityBuf HTTP streaming** - Use `HttpFCBReader` from `fcb_core` crate

---

## Task 1: Replace serde_json::Value with cjseq library

### Background
The current codebase heavily uses `serde_json::Value` for data extraction:
- **CityJSON reader** (`src/reader/cityjson.rs`): Loads entire JSON into `Value`
- **CityJSONSeq reader** (`src/reader/cjseq.rs`): Stores header+features as `Value` arrays
- **Helper functions**: All extraction functions operate on `&Value`

The `cjseq` library provides:
- In-memory representation with proper Rust types
- Enum values for CityObject types (e.g., `CityObjectType` enum)
- Better type safety and performance
- Streaming capabilities already built-in

### Current serde_json::Value Usage Locations

| File | Purpose | Change Strategy |
|------|---------|-----------------|
| `src/reader/cityjson.rs` | CityJSON format reader | Replace `Value` storage with cjseq types, use cjseq's CityJSON reader |
| `src/reader/cjseq.rs` | CityJSONSeq format reader | Replace `Value` arrays with cjseq types |
| `src/reader/remote_cityjson.rs` | Remote CityJSON reader | Use cjseq with HTTP downloading |
| `src/reader/remote_cityjsonseq.rs` | Remote CityJSONSeq reader | Use cjseq with HTTP streaming |
| `src/metadata/attributes.rs` | Attribute type inference | Update to work with cjseq types |

### Implementation Steps

#### Step 1.1: Add cjseq dependency and explore API
- [ ] Verify `cjseq = "0.4"` is in Cargo.toml (already present)
- [ ] Explore cjseq library API to understand:
  - How to read CityJSON files
  - What types are available for CityObjects, geometries, etc.
  - How to get enum values for city object types

#### Step 1.2: Update CityJSON reader (`src/reader/cityjson.rs`)
- [ ] Replace `data: RwLock<Option<Value>>` with cjseq types
- [ ] Update `ensure_loaded()` to use cjseq parser
- [ ] Update all extraction functions to use cjseq types
- [ ] Update `CityModelMetadataReader` trait implementation

#### Step 1.3: Update CityJSONSeq reader (`src/reader/cjseq.rs`)
- [ ] Replace `CityJSONSeqData` struct to use cjseq types
- [ ] Update `ensure_loaded()` to use cjseq parser
- [ ] Update extraction functions for features

#### Step 1.4: Update remote readers
- [ ] Update `src/reader/remote_cityjson.rs` to use cjseq
- [ ] Update `src/reader/remote_cityjsonseq.rs` to use cjseq

#### Step 1.5: Update attribute type inference
- [ ] Update `AttributeType::from_json_value()` to work with cjseq types
- [ ] Or create new `from_cjseq_attribute()` function

#### Step 1.6: Update tests
- [ ] Fix unit tests in each module
- [ ] Fix integration tests
- [ ] Verify all tests pass

---

## Task 2: Implement FlatCityBuf HTTP Streaming

### Background
The `fcb_core` crate provides `HttpFcbReader` for HTTP streaming:
- Efficient for remote FCB files (no full download needed)
- Supports async operations with tokio
- Can access header and features via HTTP ranges

Current state:
- Local FCB reader uses file-based `FcbReader` only
- Remote FCB reader (`src/reader/remote_flatcitybuf.rs`) partially uses HTTP

### Implementation Steps

#### Step 2.1: Update local FCB reader to support HTTP URLs
- [ ] Modify `src/reader/fcb.rs` to detect HTTP/HTTPS URLs
- [ ] When URL is HTTP, use `HttpFcbReader` instead of file-based `FcbReader`
- [ ] Handle async-to-sync bridging with tokio runtime

#### Step 2.2: Enhance remote FCB reader
- [ ] Review `src/reader/remote_flatcitybuf.rs` current implementation
- [ ] Already uses `HttpFcbReader::open()` for HTTP URLs (line ~101)
- [ ] Verify proper async handling and error propagation
- [ ] Consider if any improvements needed

#### Step 2.3: Update reader factory to support HTTP URLs
- [ ] Modify `src/reader/mod.rs` `get_reader()` function
- [ ] Detect HTTP/HTTPS URLs and route to appropriate reader
- [ ] Handle both local file paths and HTTP URLs

#### Step 2.4: Update CLI to accept HTTP URLs
- [ ] Verify CLI commands accept URLs for all formats
- [ ] Add URL detection in CLI argument handling

#### Step 2.5: Update tests
- [ ] Add tests for HTTP URL handling
- [ ] Add integration tests with mock HTTP server (if applicable)

---

## Progress Summary

| Task | Step | Status | Notes |
|------|------|--------|-------|
| Task 1 | 1.1 | Completed | Explore cjseq API |
| Task 1 | 1.2 | Completed | Update CityJSON reader with cjseq |
| Task 1 | 1.3 | Completed | Update CityJSONSeq reader with cjseq |
| Task 1 | 1.4 | Pending | Update remote readers (remote_cityjson.rs temporarily disabled) |
| Task 1 | 1.5 | Completed | Update attribute inference (works with cjseq types) |
| Task 1 | 1.6 | Pending | Update tests |
| Task 2 | 2.1 | Completed | Local FCB with HTTP - Added FcbSource enum, HTTP URL detection, async/sync bridging |
| Task 2 | 2.2 | Pending | Verify remote FCB reader |
| Task 2 | 2.3 | Completed | Update reader factory (already has get_reader_from_source for URLs) |
| Task 2 | 2.4 | Pending | Update CLI |
| Task 2 | 2.5 | Pending | Update tests |

---

## Notes

### cjseq Library Benefits
- **Type Safety**: Enum values instead of strings for city object types
- **Performance**: Efficient in-memory representation
- **Validation**: Built-in validation according to CityJSON spec
- **Streaming**: Native support for streaming large files

### HttpFcbReader Benefits
- **Efficiency**: Only downloads needed portions (header, specific features)
- **Performance**: HTTP range requests for partial access
- **Scalability**: Works well for large remote FCB files

### Potential Challenges
1. **cjseq API compatibility**: Need to understand cjseq's API surface
2. **Async/Sync bridging**: `HttpFcbReader` is async, current readers are sync
3. **Breaking changes**: May need to update trait methods
4. **Test coverage**: Need comprehensive tests for new code paths

### Files to Modify

**Task 1 (cjseq integration):**
- `src/reader/cityjson.rs`
- `src/reader/cjseq.rs`
- `src/reader/remote_cityjson.rs`
- `src/reader/remote_cityjsonseq.rs`
- `src/metadata/attributes.rs` (possibly)

**Task 2 (HTTP streaming):**
- `src/reader/fcb.rs`
- `src/reader/remote_flatcitybuf.rs` (verify/enhance)
- `src/reader/mod.rs` (factory)
- `src/cli/mod.rs` (possibly)

---

---

## Detailed Findings from cjseq Exploration

### cjseq Library API Surface

The cjseq library (v0.4.1) provides these main types:

| Type | Purpose | Fields of Interest |
|------|---------|------------------|
| `CityJSON` | Root CityJSON object | `type`, `version`, `transform`, `city_objects: HashMap`, `vertices: Vec<Vec<i64>>`, `metadata: Option<Metadata>`, `extensions` |
| `CityJSONFeature` | CityJSONSeq feature | `type`, `id`, `city_objects: HashMap`, `vertices: Vec<Vec<i64>>`, `appearance: Option<Appearance>` |
| `CityObject` | Single city object | `type: String`, `geographical_extent`, `attributes`, `geometry: Option<Geometry>`, `children`, `parents` |
| `Geometry` | Geometry data | `type: GeometryType` enum, `lod`, `boundaries: Value`, `semantics`, `material`, `texture`, `template`, `transformation_matrix` |
| `GeometryType` | Geometry type enum | `MultiPoint`, `MultiLineString`, `MultiSurface`, `CompositeSurface`, `Solid`, `MultiSolid`, `CompositeSolid`, `GeometryInstance` |
| `Metadata` | CityJSON metadata | `geographical_extent: Option<[f64; 6]>`, `identifier`, `reference_system: Option<ReferenceSystem>`, `point_of_contact` |
| `ReferenceSystem` | CRS information | `base_url`, `authority`, `version`, `code` |
| `Transform` | Coordinate transform | `scale: Vec<f64>`, `translate: Vec<f64>` |

### Key cjseq API Methods

- `CityJSON::from_str(s: &str) -> Result<CityJSON>` - Parse from string
- `CityJSON::get_metadata(&self) -> CityJSON` - Get "first line" metadata (header)
- `CityJSON::get_cjfeature(&self, i: usize) -> Option<CityJSONFeature>` - Get feature by index
- `CityJSON::number_of_city_objects(&self) -> usize` - Count all city objects
- `CityObject::get_type(&self) -> String` - Get city object type (e.g., "Building", "Road")
- `CityObject::is_toplevel(&self) -> bool` - Check if top-level object
- `GeometryType` enum provides proper typed geometry categories

### Integration Strategy

1. **Replace `Value` storage** with `CityJSON`/`CityJSONSeqData` from cjseq
2. **Use typed extraction** instead of JSON path traversal
3. **Leverage `ReferenceSystem` type** from cjseq for CRS handling
4. **Extract attributes** from `CityObject.attributes` field directly

### fcb_core HttpFcbReader

The `fcb_core` crate provides:
- `FcbReader` - File-based reader (current usage)
- `HttpFcbReader` - HTTP-based reader with async API
  - `HttpFcbReader::open(url: &str).await` - Open HTTP URL
  - Returns `header()` for metadata
  - Supports `select_query()`, `select_all_seq()` for streaming

*Last updated: 2025-02-14*
