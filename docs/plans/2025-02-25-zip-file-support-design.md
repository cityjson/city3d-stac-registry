# ZIP File Support Design

**Date:** 2025-02-25
**Status:** Approved
**Author:** Claude (Agent)

## Overview

Add support for ZIP archives containing CityJSON, CityJSONSeq, or CityGML files. The ZIP is treated as a single delivery unit that generates one STAC Item, with metadata aggregated from all supported files inside.

## Requirements

1. **Single Item Output**: A ZIP file produces one STAC Item (not a Collection)
2. **Asset Points to ZIP**: The asset `href` points to the ZIP URL
3. **ZIP Media Type**: Asset `type` is `application/zip`
4. **Internal Encoding**: `city3d:encoding` reflects internal format (CityJSON/CityGML/etc)
5. **Aggregated Metadata**: Combine bbox, object counts, LODs, and types from all files
6. **Format Priority**: CityJSON > CityJSONSeq > CityGML for encoding selection

## Architecture

```
Input Source (local .zip or remote https://...zip)
         ↓
    ZipReader
  - Extract to temp directory
  - Scan for supported files
  - Create inner readers
  - Aggregate metadata
         ↓
CityModelMetadataReader
  - bbox: Combined extent
  - city_object_count: Sum
  - lods/co_types: Union
  - encoding: Internal format
         ↓
    StacItemBuilder
  - Asset href: ZIP URL
  - Asset type: application/zip
  - city3d:encoding: Internal format
```

## Components

### New File: `src/reader/zip.rs`

```rust
pub struct ZipReader {
    file_path: PathBuf,
    temp_dir: TempDir,
    inner_readers: Vec<Box<dyn CityModelMetadataReader>>,
    metadata: RwLock<Option<ZipMetadata>>,
}

struct ZipMetadata {
    bbox: Option<BBox3D>,
    city_object_count: usize,
    city_object_types: BTreeSet<String>,
    lods: BTreeSet<String>,
    attributes: Vec<AttributeDefinition>,
    primary_encoding: &'static str,
    version: String,
    crs: Option<CRS>,
    has_textures: bool,
    has_materials: bool,
    has_semantic_surfaces: bool,
}
```

### Modified Files

| File | Changes |
|------|---------|
| `src/reader/mod.rs` | Add `.zip` case to `get_reader()` and `get_reader_from_source()` |
| `src/stac/item.rs` | Set `application/zip` for ZIP sources |
| Cargo.toml | Add `zip = "2.2"` dependency |

## Data Flow

### Local ZIP
1. User passes `data.zip`
2. `ZipReader::new("data.zip")` extracts to `/tmp/tmpXXXXX/`
3. Scan for `*.json`, `*.jsonl`, `*.gml`, `*.xml`
4. Create readers for each match
5. Aggregate metadata
6. `StacItemBuilder::from_file()` creates item with asset pointing to ZIP

### Remote ZIP
1. User passes URL
2. `download_from_url()` fetches bytes
3. Write to temp file
4. Same as local flow
5. Asset href points to original URL

## Error Handling

| Error Case | Error Type |
|------------|------------|
| Invalid ZIP | `InvalidCityJson("Invalid ZIP file")` |
| No supported files | `InvalidCityJson("No CityJSON/CityGML files found")` |
| Inner file parse error | `Other("Failed to parse file in ZIP: {filename}: {error}")` |
| CRS mismatch | Use first CRS, log warning |
| Temp directory fails | `IoError` |

## Implementation Checklist

1. Add `zip` dependency to Cargo.toml
2. Create `src/reader/zip.rs` with `ZipReader`
3. Update `src/reader/mod.rs`:
   - Import and export `ZipReader`
   - Add `.zip` case to `get_reader()`
   - Add `.zip` case to `get_reader_from_source()`
4. Update `src/stac/item.rs`:
   - Detect ZIP encoding for media type selection
5. Add tests for ZIP reader
6. Update documentation
