# CityJSON-STAC Tool - Design Document

## 1. Project Overview

### 1.1 Purpose
Create a command-line tool that generates STAC (SpatioTemporal Asset Catalog) metadata for CityJSON datasets by traversing directories and extracting metadata from various 3D city model formats.

### 1.2 Problem Statement
STAC is widely adopted for geospatial metadata but lacks native support for 3D city model formats like CityJSON. This tool bridges that gap by:
- Automatically generating STAC Items and Collections from CityJSON datasets
- Supporting multiple CityJSON-related formats
- Providing a custom STAC extension for 3D city model metadata

### 1.3 Target Formats
| Format | Extension | Status | Description |
|--------|-----------|--------|-------------|
| CityJSON | `.json` | Phase 1 | Standard CityJSON files |
| CityJSONTextSequences | `.jsonl` | Phase 1 | Line-delimited CityJSON features |
| FlatCityBuf | `.fcb` | Phase 1 | Binary columnar format for CityJSON |
| CityParquet | `.parquet` | Future | Parquet-based format (not in initial scope) |

### 1.4 Key Features
- Directory traversal and recursive scanning
- Metadata extraction from multiple formats
- STAC Item generation (one per file)
- STAC Collection generation (aggregated metadata)
- Custom STAC extension for 3D city model properties
- Static JSON output for easy web serving

## 2. Architecture Design

### 2.1 High-Level Architecture

```
┌─────────────────────────────────────────────────────────┐
│                     CLI Interface                        │
│  (clap-based argument parsing & command routing)         │
└─────────────────┬───────────────────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────────────────┐
│                Directory Traversal                       │
│  (walkdir - scan directories, filter by extension)      │
└─────────────────┬───────────────────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────────────────┐
│              Reader Factory                              │
│  (match file extension → concrete reader)               │
└─────────────────┬───────────────────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────────────────┐
│          Metadata Extraction Trait                       │
│  (CityModelMetadataReader trait interface)              │
└─────────────────┬───────────────────────────────────────┘
                  │
        ┌─────────┴──────────┬──────────────┐
        ▼                    ▼              ▼
┌──────────────┐   ┌──────────────┐   ┌──────────────┐
│  CityJSON    │   │   CityJSON   │   │ FlatCityBuf  │
│   Reader     │   │  Seq Reader  │   │   Reader     │
└──────────────┘   └──────────────┘   └──────────────┘
        │                    │              │
        └─────────┬──────────┴──────────────┘
                  ▼
┌─────────────────────────────────────────────────────────┐
│               STAC Generator                             │
│  (build STAC Items & Collections from metadata)         │
└─────────────────┬───────────────────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────────────────┐
│               JSON Serialization                         │
│  (serde_json - write STAC JSON files)                   │
└─────────────────────────────────────────────────────────┘
```

### 2.2 Design Principles

1. **Extensibility**: Use trait-based design to easily add new format readers
2. **Separation of Concerns**: Clear boundaries between reading, metadata extraction, and STAC generation
3. **Type Safety**: Leverage Rust's type system to ensure correctness
4. **Performance**: Efficient file reading with minimal memory footprint
5. **Error Handling**: Graceful degradation with informative error messages

### 2.3 Core Design Patterns

#### Factory Pattern
```rust
pub fn get_reader(file_path: &Path) -> Result<Box<dyn CityModelMetadataReader>> {
    match file_path.extension().and_then(|e| e.to_str()) {
        Some("json") => Ok(Box::new(CityJSONReader::new(file_path)?)),
        Some("jsonl") => Ok(Box::new(CityJSONSeqReader::new(file_path)?)),
        Some("fcb") => Ok(Box::new(FlatCityBufReader::new(file_path)?)),
        _ => Err(UnsupportedFormatError),
    }
}
```

#### Trait-Based Abstraction
All format readers implement a common trait for metadata extraction, ensuring consistency.

#### Builder Pattern
STAC Items and Collections will be constructed using builders for clarity and flexibility.

## 3. Module Structure

```
cityjson-stac/
├── Cargo.toml
├── README.md
├── DESIGN.md
├── STAC_EXTENSION.md
├── docs/
│   └── examples/          # Example STAC outputs
├── src/
│   ├── main.rs           # CLI entry point
│   ├── lib.rs            # Library exports
│   ├── cli/
│   │   └── mod.rs        # CLI argument parsing & commands
│   ├── reader/
│   │   ├── mod.rs        # Reader trait & factory
│   │   ├── cityjson.rs   # CityJSON reader
│   │   ├── cjseq.rs      # CityJSON Sequences reader
│   │   └── fcb.rs        # FlatCityBuf reader
│   ├── metadata/
│   │   ├── mod.rs        # Metadata structures
│   │   ├── bbox.rs       # 3D bounding box
│   │   ├── crs.rs        # Coordinate reference system
│   │   └── attributes.rs # Attribute definitions
│   ├── stac/
│   │   ├── mod.rs        # STAC module exports
│   │   ├── item.rs       # STAC Item generation
│   │   ├── collection.rs # STAC Collection generation
│   │   ├── extension.rs  # CityJSON STAC extension
│   │   └── models.rs     # STAC data models
│   ├── traversal/
│   │   └── mod.rs        # Directory traversal logic
│   └── error.rs          # Error types & handling
└── tests/
    ├── integration/
    │   └── end_to_end.rs
    └── fixtures/         # Test data files
```

## 4. Data Flow

### 4.1 Single File Processing
```
1. User provides file path
2. Factory identifies format → instantiates reader
3. Reader extracts metadata:
   - BBox (3D)
   - CRS/EPSG code
   - LODs available
   - City object types
   - Attribute schema
   - City object count
4. STAC Item builder constructs STAC JSON
5. Output written to specified location
```

### 4.2 Directory Processing
```
1. User provides directory path
2. Traversal scans recursively for supported files
3. For each file:
   a. Extract metadata (as above)
   b. Generate STAC Item
   c. Collect aggregated statistics
4. Generate STAC Collection from aggregated data
5. Write Collection JSON and all Item JSONs
6. Create catalog structure with links
```

## 5. Key Components

### 5.1 Metadata Reader Trait
```rust
pub trait CityModelMetadataReader {
    /// Get the 3D bounding box [xmin, ymin, zmin, xmax, ymax, zmax]
    fn bbox(&self) -> Result<BBox3D>;

    /// Get the coordinate reference system
    fn crs(&self) -> Result<CRS>;

    /// Get available levels of detail
    fn lods(&self) -> Result<Vec<String>>;

    /// Get city object types present in the file
    fn city_object_types(&self) -> Result<Vec<String>>;

    /// Get city object count
    fn city_object_count(&self) -> Result<usize>;

    /// Get attribute schema
    fn attributes(&self) -> Result<Vec<AttributeDefinition>>;

    /// Get encoding format name
    fn encoding(&self) -> &'static str;

    /// Get file path
    fn file_path(&self) -> &Path;
}
```

### 5.2 STAC Item Generation
Each file becomes a STAC Item with:
- Standard STAC fields (id, type, geometry, bbox, properties, assets, links)
- Custom CityJSON extension properties
- Asset pointing to the actual data file

### 5.3 STAC Collection Generation
Aggregates multiple Items:
- Collection-level metadata (extent, summaries)
- Links to all child Items
- Aggregated statistics (LOD ranges, all CO types, etc.)

## 6. Technology Stack

### 6.1 Core Dependencies
| Crate | Purpose | Justification |
|-------|---------|---------------|
| `clap` | CLI argument parsing | Industry standard, derive macros |
| `serde` + `serde_json` | JSON serialization | De facto standard for Rust JSON |
| `walkdir` | Directory traversal | Efficient recursive directory walking |
| `anyhow` | Error handling | Ergonomic error propagation |
| `thiserror` | Error definitions | Custom error types with derive |

### 6.2 Format-Specific Dependencies
| Crate | Purpose | Format |
|-------|---------|--------|
| `serde_json` | Parse CityJSON | .json |
| Custom/streaming JSON | Parse line-delimited | .jsonl |
| `flatbuffers` | Parse FlatCityBuf | .fcb |

### 6.3 Additional Utilities
- `chrono`: Timestamp handling for STAC metadata
- `url`: URL handling for STAC links
- `geojson`: GeoJSON geometry for STAC Items

## 7. Implementation Phases

### Phase 1: Core Infrastructure (Week 1)
- [ ] Project setup (Cargo.toml, module structure)
- [ ] Define core traits and metadata structures
- [ ] Implement basic CLI with clap
- [ ] Set up error handling

### Phase 2: CityJSON Reader (Week 1-2)
- [ ] Implement CityJSON reader
- [ ] Metadata extraction logic
- [ ] Unit tests with sample files

### Phase 3: STAC Generation (Week 2)
- [ ] STAC Item builder and serialization
- [ ] STAC Collection builder
- [ ] CityJSON STAC extension implementation

### Phase 4: Additional Readers (Week 2-3)
- [ ] CityJSON Sequences reader
- [ ] FlatCityBuf reader
- [ ] Reader factory implementation

### Phase 5: Directory Traversal (Week 3)
- [ ] Directory scanning logic
- [ ] Batch processing
- [ ] Collection aggregation

### Phase 6: Testing & Documentation (Week 3-4)
- [ ] Integration tests
- [ ] Example outputs
- [ ] User documentation
- [ ] CLI help text

## 8. CLI Interface Design

### 8.1 Commands

#### Generate Item
```bash
cityjson-stac item <FILE> [OPTIONS]

OPTIONS:
  -o, --output <PATH>     Output file path (default: <file>_item.json)
  --id <ID>               Custom STAC Item ID
  --title <TITLE>         Custom title
  --description <DESC>    Item description
```

#### Generate Collection
```bash
cityjson-stac collection <DIRECTORY> [OPTIONS]

OPTIONS:
  -o, --output <PATH>     Output directory (default: ./stac_output)
  --id <ID>               Collection ID
  --title <TITLE>         Collection title
  --description <DESC>    Collection description
  -r, --recursive         Scan subdirectories recursively (default: true)
  --max-depth <N>         Maximum directory depth
```

#### Validate Extension
```bash
cityjson-stac validate <STAC_FILE>

Validates STAC JSON against CityJSON extension schema
```

### 8.2 Example Usage
```bash
# Generate Item from single file
cityjson-stac item building.json -o building_stac.json

# Generate Collection from directory
cityjson-stac collection ./data/ \
  --title "City Buildings Dataset" \
  --description "Building models in LOD2" \
  -o ./stac_catalog

# Process with custom options
cityjson-stac collection ./data/ \
  --recursive \
  --max-depth 3 \
  --id "city-buildings-2024"
```

## 9. Configuration

### 9.1 Config File (Optional)
`cityjson-stac.toml`:
```toml
[stac]
version = "1.0.0"
license = "CC-BY-4.0"

[extension]
prefix = "cj"
version = "1.0.0"

[output]
pretty_print = true
include_self_links = true
```

## 10. Performance Considerations

### 10.1 Optimization Strategies
1. **Streaming**: Read large files incrementally (especially .jsonl)
2. **Parallel Processing**: Use `rayon` for parallel file processing in directories
3. **Memory Management**: Avoid loading entire files into memory
4. **Caching**: Cache parsed metadata during collection aggregation

### 10.2 Expected Performance
- Single file: < 1 second for typical CityJSON files (< 100MB)
- Directory (100 files): < 30 seconds
- Large collections (1000+ files): Use parallel processing

## 11. Error Handling Strategy

### 11.1 Error Categories
```rust
#[derive(thiserror::Error, Debug)]
pub enum CityJsonStacError {
    #[error("Unsupported file format: {0}")]
    UnsupportedFormat(String),

    #[error("Failed to read file: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Failed to parse metadata: {0}")]
    ParseError(String),

    #[error("Invalid STAC structure: {0}")]
    StacError(String),

    #[error("Missing required field: {0}")]
    MissingField(String),
}
```

### 11.2 Graceful Degradation
- Skip unreadable files with warnings
- Continue collection generation even if some items fail
- Log errors but don't abort entire process

## 12. Testing Strategy

### 12.1 Unit Tests
- Each reader implementation
- Metadata extraction logic
- STAC builders

### 12.2 Integration Tests
- End-to-end file → STAC Item
- Directory → STAC Collection
- Multiple format handling

### 12.3 Test Fixtures
- Sample CityJSON files (various LODs, CRS)
- CityJSON Sequences samples
- FlatCityBuf samples
- Invalid files for error handling

## 13. Future Enhancements

### 13.1 Phase 2 Features
- [ ] CityParquet support
- [ ] Parallel processing for large directories
- [ ] STAC API server mode (dynamic catalog)
- [ ] Watch mode for incremental updates
- [ ] Spatial filtering during traversal

### 13.2 Advanced Features
- [ ] Thumbnail generation from 3D models
- [ ] Geometry simplification for STAC geometries
- [ ] Semantic validation of CityJSON attributes
- [ ] Integration with STAC validators
- [ ] Cloud storage support (S3, Azure Blob)

## 14. References

- STAC Specification: https://stacspec.org/
- CityJSON: https://www.cityjson.org/
- CityJSON Sequences: https://github.com/cityjson/cjseq
- FlatCityBuf: https://github.com/cityjson/flatcitybuf
