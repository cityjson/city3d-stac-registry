# Technical Design Document

This document provides detailed technical architecture and implementation guidelines for the cityjson-stac tool.

## Table of Contents

1. [Architecture Details](#architecture-details)
2. [Design Philosophy](#design-philosophy)
3. [Development Standards](#development-standards)
4. [Test Data](#test-data)
5. [Module Design](#module-design)
6. [Data Flow](#data-flow)
7. [CLI Specification](#cli-specification)
8. [Error Handling](#error-handling)
9. [Testing Strategy](#testing-strategy)

---

## Architecture Details

### Design Principles

1. **Trait-Based Extensibility**: New format readers implement `CityModelMetadataReader`
2. **Separation of Concerns**: Clear boundaries between reading, metadata extraction, and STAC generation
3. **Type Safety**: Leverage Rust's type system for correctness
4. **Performance**: Lazy loading and optional parallel processing

### Core Trait Definition

```rust
/// Common interface for all CityJSON-format readers
pub trait CityModelMetadataReader: Send + Sync {
    /// Extract 3D bounding box [xmin, ymin, zmin, xmax, ymax, zmax]
    fn bbox(&self) -> Result<BBox3D>;

    /// Get coordinate reference system
    fn crs(&self) -> Result<CRS>;

    /// Get available levels of detail
    fn lods(&self) -> Result<Vec<String>>;

    /// Get city object types present
    fn city_object_types(&self) -> Result<Vec<String>>;

    /// Count total city objects
    fn city_object_count(&self) -> Result<usize>;

    /// Extract attribute schema definitions
    fn attributes(&self) -> Result<Vec<AttributeDefinition>>;

    /// Get encoding format name
    fn encoding(&self) -> &'static str;

    /// Get CityJSON version
    fn version(&self) -> Result<String>;

    /// Get file path
    fn file_path(&self) -> &Path;

    /// Get coordinate transform parameters
    fn transform(&self) -> Result<Option<Transform>>;

    /// Extract additional metadata
    fn metadata(&self) -> Result<Option<serde_json::Value>>;
}
```

### Reader Factory Pattern

```rust
/// Factory function to create appropriate reader for a file
pub fn get_reader(file_path: &Path) -> Result<Box<dyn CityModelMetadataReader>> {
    let extension = file_path
        .extension()
        .and_then(|e| e.to_str())
        .ok_or_else(|| CityJsonStacError::UnsupportedFormat("No extension".into()))?;

    match extension.to_lowercase().as_str() {
        "json" => Ok(Box::new(CityJSONReader::new(file_path)?)),
        "jsonl" | "cjseq" => Ok(Box::new(CityJSONSeqReader::new(file_path)?)),
        "fcb" => Ok(Box::new(FlatCityBufReader::new(file_path)?)),
        _ => Err(CityJsonStacError::UnsupportedFormat(extension.into())),
    }
}
```

---

## Design Philosophy

### Interface Programming

The codebase follows a **trait-oriented design** where all format readers implement the `CityModelMetadataReader` trait. This provides:

1. **Polymorphism**: Callers work with `dyn CityModelMetadataReader` without knowing the concrete implementation
2. **Extensibility**: New formats can be added by implementing the trait without changing existing code
3. **Testability**: Mock implementations can be easily created for testing

### Factory Pattern

The `get_reader()` function serves as a factory that:
- Examines file extensions to determine format
- Returns a boxed trait object (`Box<dyn CityModelMetadataReader>`)
- Encapsulates reader creation logic
- Makes the codebase open for extension (new readers) but closed for modification

### Streaming-First Approach

Different formats have different characteristics:
- **CityJSON (`.json`)**: Single-file format, typically small enough to load entirely
- **CityJSONSeq (`.jsonl`)**: Designed for streaming, processes line-by-line
- **FlatCityBuf (`.fcb`)**: Binary format with header metadata that can be read without full file load

The reader implementations respect these characteristics:
- Use lazy loading with `RwLock` for interior mutability
- Stream `.jsonl` files line-by-line when possible
- Cache extracted metadata to avoid redundant computation

### Data Source Abstraction

The `InputSource` enum abstracts between local and remote data:

```rust
pub enum InputSource {
    Local(PathBuf),
    Remote(String),  // http://, https://, s3://, az://, gs://
}
```

This design allows:
- Future support for HTTP/HTTPS and object storage (S3, Azure, GCS)
- Transparent switching between local and remote data
- Consistent API regardless of data location

### Module Dependency Direction

```
┌─────────────────────────────────────────────────────────┐
│                    STAC Module                          │
│          (consumes metadata from readers)               │
└────────────────────┬────────────────────────────────────┘
                     │ depends on
                     ▼
┌─────────────────────────────────────────────────────────┐
│                   Reader Module                         │
│   (implements CityModelMetadataReader trait)            │
└────────────────────┬────────────────────────────────────┘
                     │ depends on
                     ▼
┌─────────────────────────────────────────────────────────┐
│                 Metadata Module                         │
│       (BBox3D, CRS, Transform, AttributeDefinition)     │
└─────────────────────────────────────────────────────────┘
```

**Key principle**: Readers depend on metadata types, not vice versa. The metadata module has no knowledge of readers or CityJSON formats.

---

## Development Standards

### Test-Driven Development (TDD)

When adding new features or modifying existing ones:
1. Write tests first to define expected behavior
2. Run tests to see them fail (red)
3. Implement the minimum code to make tests pass (green)
4. Refactor while keeping tests green

### Code Quality Standards

**Pre-commit checklist** (enforced by git hooks):
```bash
# Format code
cargo fmt

# Check for lints and warnings
cargo clippy -- -D warnings

# Run unit tests
cargo test --lib
```

**Rules**:
- **NEVER** use `#[allow(dead_code)]` or `#[allow(unused)]` as workarounds
- If code is truly not needed, remove it
- If clippy warns about dead code during development, consider marking tests as `#[cfg(test)]`
- If clippy warns about unused code that will be used later, consider using `#[expect(dead_code)]` with a comment explaining why

### Testing Guidelines

1. **Return mocked data in tests, not real data**
   - Tests should be deterministic and fast
   - Create minimal test fixtures inline in the test function
   - Avoid reading external files in unit tests (use integration tests for file I/O)

2. **Use descriptive test names**
   ```rust
   #[test]
   fn test_bbox_merge_returns_union_of_both_boxes() { ... }
   ```

3. **Test both success and failure paths**
   - Verify error handling with invalid inputs
   - Check that error messages are helpful

4. **Organize tests by module**
   - Unit tests in `src/<module>/tests.rs` or inline `#[cfg(test)]` modules
   - Integration tests in `tests/` directory

---

## Test Data

### Test Data Location

Test fixtures are located in `tests/data/`:

| File | Format | Description | Use Case |
|------|--------|-------------|----------|
| `delft.city.json` | CityJSON | Small file with metadata but no geometry | Basic reader tests |
| `delft.city.jsonl` | CityJSONSeq | 159 lines (1 header + 158 features) | Streaming tests |
| `railway.city.json` | CityJSON | Complex file with geometry and attributes | Full metadata extraction |
| `railway.city.jsonl` | CityJSONSeq | 39 lines (1 header + 38 features) | Smaller streaming dataset |
| `all.fcb` | FlatCityBuf | Binary format reference | FCB reader tests |

### Using Test Data in Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn test_data_path(file: &str) -> PathBuf {
        PathBuf::from("tests/data").join(file)
    }

    #[test]
    fn test_delft_cityjson() {
        let path = test_data_path("delft.city.json");
        let reader = CityJSONReader::new(&path).unwrap();
        // ... test assertions
    }
}
```

### Adding New Test Files

When adding new test data:
1. Place the file in `tests/data/`
2. Document its purpose in the table above
3. Add a corresponding test that uses it
4. Keep test files minimal but representative of real-world data

---

## Module Design

### Metadata Structures

#### BBox3D

```rust
/// 3D Bounding box [xmin, ymin, zmin, xmax, ymax, zmax]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BBox3D {
    pub xmin: f64,
    pub ymin: f64,
    pub zmin: f64,
    pub xmax: f64,
    pub ymax: f64,
    pub zmax: f64,
}

impl BBox3D {
    /// Merge two bounding boxes (union)
    pub fn merge(&self, other: &BBox3D) -> BBox3D;

    /// Convert to STAC bbox array format
    pub fn to_array(&self) -> [f64; 6];

    /// Get 2D footprint (for STAC geometry)
    pub fn footprint_2d(&self) -> [f64; 4];
}
```

#### CRS (Coordinate Reference System)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CRS {
    /// EPSG code (e.g., 7415)
    pub epsg: Option<u32>,
    /// WKT2 representation
    pub wkt2: Option<String>,
    /// CityJSON authority/identifier
    pub authority: Option<String>,
    pub identifier: Option<String>,
}
```

#### Transform

```rust
/// Coordinate transform for vertex compression
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transform {
    pub scale: [f64; 3],
    pub translate: [f64; 3],
}
```

#### AttributeDefinition

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributeDefinition {
    pub name: String,
    #[serde(rename = "type")]
    pub attr_type: AttributeType,
    pub description: Option<String>,
    pub required: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AttributeType {
    String, Number, Boolean, Date, Array, Object,
}
```

### STAC Builders

#### StacItemBuilder

```rust
pub struct StacItemBuilder {
    id: String,
    geometry: Option<Geometry>,
    bbox: Option<Vec<f64>>,
    properties: serde_json::Map<String, Value>,
    assets: serde_json::Map<String, Value>,
    links: Vec<Link>,
}

impl StacItemBuilder {
    pub fn new(id: impl Into<String>) -> Self;
    pub fn bbox(self, bbox: BBox3D) -> Self;
    pub fn geometry(self, geom: Geometry) -> Self;
    pub fn datetime(self, dt: DateTime<Utc>) -> Self;
    pub fn title(self, title: impl Into<String>) -> Self;
    pub fn cityjson_metadata(self, reader: &dyn CityModelMetadataReader) -> Result<Self>;
    pub fn data_asset(self, href: String, media_type: &str) -> Self;
    pub fn build(self) -> Result<StacItem>;
}
```

#### StacCollectionBuilder

```rust
pub struct StacCollectionBuilder {
    id: String,
    title: Option<String>,
    description: Option<String>,
    license: String,
    extent: Extent,
    summaries: serde_json::Map<String, Value>,
    links: Vec<Link>,
}

impl StacCollectionBuilder {
    pub fn new(id: impl Into<String>) -> Self;
    pub fn spatial_extent(self, bbox: BBox3D) -> Self;
    pub fn temporal_extent(self, start: Option<DateTime<Utc>>, end: Option<DateTime<Utc>>) -> Self;
    pub fn aggregate_cityjson_metadata(self, readers: &[Box<dyn CityModelMetadataReader>]) -> Result<Self>;
    pub fn build(self) -> Result<StacCollection>;
}
```

---

## Data Flow

### Single File Processing

```
User Command: cityjson-stac item building.json
     │
     ▼
Parse CLI Arguments
     │
     ├── file: "building.json"
     ├── output: None (use default)
     │
     ▼
Reader Factory
     │
     ├── Check extension: ".json"
     ├── Create CityJSONReader
     │
     ▼
Extract Metadata
     │
     ├── Load JSON (lazy)
     ├── Extract bbox from vertices or metadata
     ├── Extract CRS from referenceSystem
     ├── Scan geometry for LODs
     ├── Collect city object types
     ├── Count objects
     ├── Build attribute schema
     │
     ▼
Build STAC Item
     │
     ├── StacItemBuilder::new()
     ├── .bbox(metadata.bbox)
     ├── .cityjson_metadata(&reader)
     ├── .data_asset(path, media_type)
     ├── .build()
     │
     ▼
Write Output → building_item.json
```

### Directory Processing

```
User Command: cityjson-stac collection ./buildings/
     │
     ▼
Directory Traversal (walkdir)
     │
     ├── Filter by extensions: [json, jsonl, fcb]
     ├── Respect max-depth if set
     │
     ▼
Process Each File
     │
     ├── Create reader
     ├── Extract metadata
     ├── Build STAC Item
     ├── Store for aggregation
     │
     ▼
Aggregate Metadata
     │
     ├── Merge all bboxes → collection bbox
     ├── Union all LODs
     ├── Union all city object types
     ├── Union encodings
     ├── Calculate count statistics
     │
     ▼
Generate Collection + Items
     │
     ├── Write collection.json
     ├── Write items/*.json
     │
     ▼
Output Structure:
    stac_output/
    ├── collection.json
    └── items/
        ├── building_001_item.json
        └── ...
```

---

## CLI Specification

### Command: `item`

Generate STAC Item from a single file.

```
cityjson-stac item <FILE> [OPTIONS]

Arguments:
  <FILE>                    Input file path (.json, .jsonl, .fcb)

Options:
  -o, --output <PATH>       Output file path [default: <file>_item.json]
      --id <ID>             Custom STAC Item ID [default: filename]
      --title <TITLE>       Item title
  -d, --description <DESC>  Item description
      --datetime <ISO8601>  Dataset timestamp [default: now]
  -c, --collection <ID>     Parent collection ID
  -l, --license <LICENSE>   Data license [default: proprietary]
      --pretty              Pretty-print JSON [default: true]
  -v, --verbose             Verbose output
```

### Command: `collection`

Generate STAC Collection from a directory.

```
cityjson-stac collection <DIRECTORY> [OPTIONS]

Arguments:
  <DIRECTORY>               Directory to scan

Options:
  -o, --output <PATH>       Output directory [default: ./stac_output]
      --id <ID>             Collection ID [default: directory name]
      --title <TITLE>       Collection title
  -d, --description <DESC>  Collection description
  -l, --license <LICENSE>   Data license [default: proprietary]
  -r, --recursive           Scan subdirectories [default: true]
      --max-depth <N>       Maximum directory depth
  -e, --extensions <EXT>    File extensions to include [default: all]
  -p, --parallel            Enable parallel processing
      --skip-errors         Skip files with errors [default: true]
      --pretty              Pretty-print JSON [default: true]
  -v, --verbose             Verbose output
```

### Exit Codes

| Code | Meaning                   |
| ---- | ------------------------- |
| 0    | Success                   |
| 1    | File/directory not found  |
| 2    | Unsupported format        |
| 3    | Metadata extraction error |
| 4    | Output write error        |

---

## Error Handling

### Error Types

```rust
#[derive(Error, Debug)]
pub enum CityJsonStacError {
    #[error("Unsupported file format: {0}")]
    UnsupportedFormat(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("JSON parsing error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Failed to extract metadata: {0}")]
    MetadataError(String),

    #[error("Invalid CityJSON structure: {0}")]
    InvalidCityJson(String),

    #[error("Missing required field: {0}")]
    MissingField(String),

    #[error("STAC generation error: {0}")]
    StacError(String),
}
```

### Error Handling Strategy

1. **Graceful degradation**: Continue processing other files if one fails
2. **Informative messages**: Include file path and specific error details
3. **Skip-errors mode**: Default behavior for directory processing

---

## Testing Strategy

### Test Pyramid

```
       /\
      /E2E\         End-to-end CLI tests
     /------\
    /  INT   \      Module integration tests
   /----------\
  /    UNIT    \    Function-level tests
 /--------------\
```

### Test Categories

1. **Unit Tests**: Individual functions in each module
2. **Integration Tests**: Reader → STAC builder flow
3. **E2E Tests**: Full CLI command execution

### Test Fixtures

Located in `tests/fixtures/`:

- `simple_building.json` - Minimal valid CityJSON
- `complex_city.json` - Multiple object types, LODs
- `invalid_*.json` - Error handling tests

### Coverage Target

- Unit tests: 80%+ coverage
- Integration tests: Critical paths
- E2E tests: All CLI commands

---

## FlatCityBuf Integration

For `.fcb` file support, use the [FlatCityBuf](https://github.com/cityjson/flatcitybuf) Rust library:

```rust
// Add to Cargo.toml dependencies:
// flatcitybuf = { git = "https://github.com/cityjson/flatcitybuf" }

use flatcitybuf::FcbReader;

pub struct FlatCityBufReader {
    file_path: PathBuf,
    reader: FcbReader,
}

impl CityModelMetadataReader for FlatCityBufReader {
    fn encoding(&self) -> &'static str {
        "FlatCityBuf"
    }
    // ... implement other trait methods using FcbReader API
}
```

The FlatCityBuf library provides efficient binary reading with spatial indexing support. See the [repository](https://github.com/cityjson/flatcitybuf) for API documentation.

---

## Performance Considerations

### Lazy Loading

Readers should defer file parsing until metadata is actually requested:

```rust
pub struct CityJSONReader {
    file_path: PathBuf,
    data: Option<Value>,  // Loaded on first access
}

impl CityJSONReader {
    fn ensure_loaded(&mut self) -> Result<&Value> {
        if self.data.is_none() {
            let file = File::open(&self.file_path)?;
            self.data = Some(serde_json::from_reader(BufReader::new(file))?);
        }
        Ok(self.data.as_ref().unwrap())
    }
}
```

### Parallel Processing

Optional parallel file processing for large directories:

```rust
#[cfg(feature = "parallel")]
use rayon::prelude::*;

// With --parallel flag:
files.par_iter()
    .map(|f| get_reader(f).and_then(|r| process(r)))
    .collect::<Vec<_>>()
```

### Memory Management

- Stream large `.jsonl` files line-by-line
- Use FlatCityBuf header for metadata without loading full file
- Limit concurrent readers in parallel mode
