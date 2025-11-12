# CityJSON-STAC Project Summary

## Quick Overview

**Project:** CityJSON-STAC CLI Tool
**Purpose:** Generate STAC metadata for 3D city model datasets
**Language:** Rust
**Status:** Design Phase Complete, Ready for Implementation

## What This Tool Does

Automatically creates STAC (SpatioTemporal Asset Catalog) metadata files from CityJSON datasets:

```
Input:                          Output:
┌─────────────────┐            ┌──────────────────┐
│ building.json   │────────────>│ STAC Item JSON   │
│ (CityJSON)      │            │ (with metadata)  │
└─────────────────┘            └──────────────────┘

Input:                          Output:
┌─────────────────┐            ┌──────────────────┐
│ ./buildings/    │            │ STAC Collection  │
│ ├── file1.json  │────────────>│ ├── collection.json
│ ├── file2.jsonl │            │ └── items/       │
│ └── file3.fcb   │            │     ├── item1.json
└─────────────────┘            │     ├── item2.json
                               │     └── item3.json
                               └──────────────────┘
```

## Supported Formats

| Format | Extension | Status |
|--------|-----------|--------|
| CityJSON | `.json` | Phase 1 |
| CityJSON Text Sequences | `.jsonl` | Phase 1 |
| FlatCityBuf | `.fcb` | Phase 1 |
| CityParquet | `.parquet` | Future |

## Core Architecture

### Design Pattern: Trait-Based Factory

```rust
// Common interface for all formats
trait CityModelMetadataReader {
    fn bbox() -> BBox3D;
    fn lods() -> Vec<String>;
    fn city_object_types() -> Vec<String>;
    // ... more methods
}

// Factory creates appropriate reader
fn get_reader(path: &Path) -> Box<dyn CityModelMetadataReader> {
    match extension {
        "json" => CityJSONReader,
        "jsonl" => CityJSONSeqReader,
        "fcb" => FlatCityBufReader,
    }
}
```

### Data Flow

```
File Input
    ↓
Reader Factory (identifies format)
    ↓
Format-Specific Reader (extracts metadata)
    ↓
STAC Builder (constructs STAC JSON)
    ↓
JSON Output
```

## Key Features

### Metadata Extraction

Automatically extracts from CityJSON files:
- **3D Bounding Box** (xmin, ymin, zmin, xmax, ymax, zmax)
- **CRS/EPSG Code** (coordinate reference system)
- **LODs** (levels of detail: 0, 1, 2, 3)
- **City Object Types** (Building, Road, TINRelief, etc.)
- **Object Count** (number of city objects)
- **Attributes Schema** (semantic attributes definition)
- **Coordinate Transform** (for compressed coordinates)

### STAC Extension

Custom extension prefix: `cj:`

Item-level properties:
```json
{
  "cj:encoding": "CityJSON",
  "cj:version": "2.0",
  "cj:city_objects": 1523,
  "cj:lods": ["2", "2.2"],
  "cj:co_types": ["Building", "BuildingPart"],
  "cj:attributes": [...],
  "cj:transform": {...}
}
```

Collection-level summaries:
```json
{
  "cj:encoding": ["CityJSON", "FlatCityBuf"],
  "cj:lods": ["0", "1", "2", "3"],
  "cj:city_objects": {
    "min": 45,
    "max": 5234,
    "total": 125432
  }
}
```

## CLI Commands

### Generate STAC Item
```bash
cityjson-stac item building.json -o building_item.json
```

### Generate STAC Collection
```bash
cityjson-stac collection ./buildings/ \
  --title "City Buildings" \
  --description "LOD2 building models" \
  -o ./stac_catalog
```

### Validate STAC
```bash
cityjson-stac validate building_item.json
```

## Module Structure

```
cityjson-stac/
├── src/
│   ├── cli/           # Command-line interface
│   ├── reader/        # Format readers
│   │   ├── cityjson.rs
│   │   ├── cjseq.rs
│   │   └── fcb.rs
│   ├── metadata/      # Data structures
│   │   ├── bbox.rs
│   │   ├── crs.rs
│   │   └── attributes.rs
│   ├── stac/          # STAC generation
│   │   ├── item.rs
│   │   ├── collection.rs
│   │   └── extension.rs
│   ├── traversal/     # Directory scanning
│   └── error.rs       # Error handling
```

## Implementation Timeline

**Total Duration:** 3-4 weeks

| Week | Focus | Deliverables |
|------|-------|--------------|
| 1 | Foundation + CityJSON Reader | Core structures, CityJSON support |
| 2 | STAC Generation + Additional Formats | STAC builders, CityJSONSeq, FlatCityBuf |
| 3 | CLI + Directory Traversal | Complete CLI, collection generation |
| 4 | Testing + Documentation | Tests, docs, release prep |

## Key Dependencies

```toml
[dependencies]
clap = "4.5"              # CLI framework
serde = "1.0"             # Serialization
serde_json = "1.0"        # JSON handling
walkdir = "2.4"           # Directory traversal
anyhow = "1.0"            # Error handling
thiserror = "1.0"         # Custom errors
chrono = "0.4"            # Date/time
geojson = "0.24"          # GeoJSON geometry
flatbuffers = "24.3"      # FlatCityBuf support
```

## Documentation Structure

| Document | Purpose |
|----------|---------|
| `README.md` | Project overview and quick start |
| `DESIGN.md` | Architecture and design decisions |
| `STAC_EXTENSION.md` | STAC extension specification |
| `API_DESIGN.md` | Trait definitions and API contracts |
| `CLI_DESIGN.md` | Command-line interface details |
| `CARGO_STRUCTURE.md` | Dependencies and build configuration |
| `IMPLEMENTATION_PLAN.md` | Phase-by-phase implementation guide |
| `PROJECT_SUMMARY.md` | This document - high-level overview |

## Example Output

### STAC Item (simplified)
```json
{
  "type": "Feature",
  "id": "rotterdam_buildings",
  "bbox": [4.46, 51.91, -5.0, 4.49, 51.93, 100.0],
  "properties": {
    "datetime": "2023-05-15T00:00:00Z",
    "proj:epsg": 7415,
    "cj:encoding": "CityJSON",
    "cj:city_objects": 1523,
    "cj:lods": ["2", "2.2"],
    "cj:co_types": ["Building", "BuildingPart"]
  },
  "assets": {
    "data": {
      "href": "./rotterdam_buildings.json",
      "type": "application/json"
    }
  }
}
```

### STAC Collection (simplified)
```json
{
  "type": "Collection",
  "id": "rotterdam_3dcity_2023",
  "title": "Rotterdam 3D City Model 2023",
  "extent": {
    "spatial": {
      "bbox": [[4.42, 51.88, -5.0, 4.60, 51.98, 120.5]]
    }
  },
  "summaries": {
    "cj:encoding": ["CityJSON", "FlatCityBuf"],
    "cj:lods": ["0", "1", "2", "3"],
    "cj:city_objects": {
      "min": 45,
      "max": 5234,
      "total": 125432
    }
  }
}
```

## Design Principles

1. **Extensibility** - Easy to add new format readers via traits
2. **Type Safety** - Leverage Rust's type system
3. **Performance** - Efficient file processing, optional parallelization
4. **Error Handling** - Graceful degradation, informative messages
5. **Separation of Concerns** - Clear module boundaries

## Testing Strategy

```
Test Pyramid:
       /\
      /E2E\         End-to-end workflows
     /------\
    /  INT   \      Module integration
   /----------\
  /    UNIT    \    Individual functions
 /--------------\
```

**Coverage Goal:** 80%+ unit test coverage

**Test Fixtures:**
- Small CityJSON (~1MB)
- Large CityJSON (~100MB)
- CityJSONSeq files
- FlatCityBuf files
- Invalid files (error handling)

## Success Metrics

### Minimum Viable Product (MVP)
- ✅ Design documentation complete
- ⬜ CityJSON format support
- ⬜ STAC Item generation
- ⬜ STAC Collection generation
- ⬜ CLI with item/collection commands
- ⬜ Published to crates.io

### Performance Targets
- Single file (10MB): < 1 second
- Directory (100 files): < 30 seconds
- Memory usage: < 500MB typical

## Next Steps

1. **Initialize Rust Project**
   ```bash
   cargo new cityjson-stac
   cd cityjson-stac
   ```

2. **Set Up Project Structure**
   - Create module directories
   - Add dependencies to Cargo.toml
   - Set up CI/CD (GitHub Actions)

3. **Start Implementation (Phase 1)**
   - Implement core data structures
   - Define reader trait
   - Set up error handling

4. **Iterative Development**
   - Follow implementation plan
   - Test-driven development
   - Regular commits

## References

### Specifications
- [STAC Specification](https://stacspec.org/) - Main STAC spec
- [CityJSON Specification](https://www.cityjson.org/specs/) - CityJSON spec
- [STAC Extensions](https://stac-extensions.github.io/) - Extension examples

### Related Projects
- [CityJSON Sequences](https://github.com/cityjson/cjseq) - Line-delimited format
- [FlatCityBuf](https://github.com/cityjson/flatcitybuf) - Columnar format
- [3D BAG](https://3dbag.nl) - Dutch 3D building dataset (sample data)

### Tools
- [STAC Browser](https://radiantearth.github.io/stac-browser/) - Browse STAC catalogs
- [STAC Validator](https://github.com/stac-utils/stac-validator) - Validate STAC
- [CityJSON Validator](https://validator.cityjson.org/) - Validate CityJSON

## Quick Start (After Implementation)

```bash
# Install
cargo install cityjson-stac

# Generate item
cityjson-stac item building.json

# Generate collection
cityjson-stac collection ./data/

# Validate
cityjson-stac validate item.json
```

## Contributing

The project welcomes contributions:
- Format reader implementations
- STAC extension enhancements
- Performance optimizations
- Documentation improvements
- Bug fixes

## License

[To be determined - suggest MIT or Apache-2.0]

---

**Project Status:** 📋 Design Complete → 🚧 Ready for Implementation

**Estimated Completion:** 3-4 weeks from start

**Primary Contact:** [To be filled in]

**Repository:** https://github.com/cityjson/cityjson-stac (proposed)
