# Architecture Overview

## System Architecture Diagram

```
┌──────────────────────────────────────────────────────────────────────┐
│                         USER INTERFACE                                │
│                                                                        │
│  Command Line Interface (clap)                                        │
│  ┌────────────┐  ┌────────────────┐  ┌─────────────┐                │
│  │   item     │  │  collection    │  │  validate   │                 │
│  │  command   │  │   command      │  │   command   │                 │
│  └─────┬──────┘  └────────┬───────┘  └──────┬──────┘                │
└────────┼──────────────────┼──────────────────┼─────────────────────────┘
         │                  │                  │
         ▼                  ▼                  │
┌────────────────────────────────────────────┐│
│         FILE/DIRECTORY DISCOVERY           ││
│                                            ││
│  Single File           Directory Traversal ││
│  Validation           (walkdir)            ││
│                       - Recursive scan     ││
│                       - Filter by ext      ││
│                       - Max depth          ││
└────────────────┬───────────────────────────┘│
                 │                            │
                 ▼                            │
┌─────────────────────────────────────────────┐
│         READER FACTORY                      │
│                                             │
│  get_reader(path: &Path)                   │
│    ├─> Check file extension                │
│    ├─> Validate file format                │
│    └─> Instantiate appropriate reader      │
└────────────────┬────────────────────────────┘
                 │
        ┌────────┴──────────┬──────────────┐
        │                   │              │
        ▼                   ▼              ▼
┌──────────────┐   ┌──────────────┐   ┌──────────────┐
│  CityJSON    │   │ CityJSONSeq  │   │ FlatCityBuf  │
│   Reader     │   │   Reader     │   │   Reader     │
│              │   │              │   │              │
│ .json files  │   │ .jsonl files │   │  .fcb files  │
└──────┬───────┘   └──────┬───────┘   └──────┬───────┘
       │                  │                  │
       │  Implements      │                  │
       └──────────────────┴──────────────────┘
                          │
                          ▼
        ┌──────────────────────────────────────────────┐
        │   CityModelMetadataReader Trait              │
        │                                              │
        │   - bbox() -> BBox3D                        │
        │   - crs() -> CRS                            │
        │   - lods() -> Vec<String>                   │
        │   - city_object_types() -> Vec<String>      │
        │   - city_object_count() -> usize            │
        │   - attributes() -> Vec<AttributeDefinition>│
        │   - encoding() -> &str                      │
        │   - version() -> String                     │
        │   - transform() -> Option<Transform>        │
        │   - metadata() -> Option<Value>             │
        └────────────────────┬─────────────────────────┘
                             │
                             ▼
        ┌──────────────────────────────────────────────┐
        │         METADATA STRUCTURES                  │
        │                                              │
        │  BBox3D    CRS    Transform                 │
        │  AttributeDefinition                         │
        └────────────────────┬─────────────────────────┘
                             │
                             ▼
        ┌──────────────────────────────────────────────┐
        │         STAC GENERATION                      │
        │                                              │
        │  ┌────────────────┐   ┌──────────────────┐ │
        │  │ StacItemBuilder│   │StacCollectionBuilder│
        │  │                │   │                  │ │
        │  │ - id, bbox     │   │ - id, extent     │ │
        │  │ - properties   │   │ - summaries      │ │
        │  │ - assets       │   │ - aggregation    │ │
        │  │ - links        │   │ - links          │ │
        │  └────────┬───────┘   └────────┬─────────┘ │
        └───────────┼──────────────────────┼───────────┘
                    │                      │
                    ▼                      ▼
        ┌────────────────────┐   ┌──────────────────────┐
        │    STAC Item       │   │  STAC Collection     │
        │                    │   │                      │
        │  - Feature         │   │  - Collection        │
        │  - cj:* properties │   │  - cj:* summaries    │
        │  - Asset refs      │   │  - Item links        │
        └────────┬───────────┘   └────────┬─────────────┘
                 │                        │
                 ▼                        ▼
        ┌──────────────────────────────────────────────┐
        │         JSON SERIALIZATION                   │
        │                                              │
        │  serde_json::to_string_pretty()             │
        └────────────────────┬─────────────────────────┘
                             │
                             ▼
        ┌──────────────────────────────────────────────┐
        │         FILE OUTPUT                          │
        │                                              │
        │  Item:       item.json                      │
        │  Collection: collection.json + items/       │
        └──────────────────────────────────────────────┘
```

## Component Interactions

### Single File Processing Flow

```
User Command
    │
    ├─> cityjson-stac item building.json
    │
    ▼
Parse CLI Arguments
    │
    ├─> file: "building.json"
    ├─> output: None (use default)
    ├─> title: None
    │
    ▼
Validate File Exists
    │
    ├─> Check path exists
    ├─> Check read permissions
    │
    ▼
Reader Factory
    │
    ├─> Check extension: ".json"
    ├─> Verify CityJSON format
    ├─> Create CityJSONReader
    │
    ▼
Extract Metadata
    │
    ├─> Load JSON (lazy)
    ├─> Extract bbox
    ├─> Extract CRS/EPSG
    ├─> Scan for LODs
    ├─> Collect city object types
    ├─> Count objects
    ├─> Build attribute schema
    ├─> Get transform
    │
    ▼
Build STAC Item
    │
    ├─> StacItemBuilder::new()
    ├─> .bbox(metadata.bbox)
    ├─> .cityjson_metadata(&reader)
    ├─> .data_asset(path, "application/json")
    ├─> .build()
    │
    ▼
Serialize to JSON
    │
    ├─> serde_json::to_string_pretty()
    │
    ▼
Write Output
    │
    ├─> building_item.json
    │
    ▼
Success Message
    │
    └─> "✓ Generated STAC Item: building_item.json"
```

### Directory Processing Flow

```
User Command
    │
    ├─> cityjson-stac collection ./buildings/
    │
    ▼
Parse CLI Arguments
    │
    ├─> directory: "./buildings/"
    ├─> recursive: true
    ├─> output: "./stac_output"
    │
    ▼
Directory Traversal
    │
    ├─> walkdir::WalkDir::new(dir)
    ├─> Filter by extensions: [json, jsonl, fcb]
    ├─> Respect max-depth
    │
    ├─> Found: building_001.json
    ├─> Found: building_002.json
    ├─> Found: terrain.fcb
    ├─> ... (total: 156 files)
    │
    ▼
Process Each File
    │
    ├─> For building_001.json:
    │   ├─> get_reader()
    │   ├─> Extract metadata
    │   ├─> Store reader
    │
    ├─> For building_002.json:
    │   ├─> get_reader()
    │   ├─> Extract metadata
    │   ├─> Store reader
    │
    ├─> For terrain.fcb:
    │   ├─> get_reader()
    │   ├─> Extract metadata
    │   ├─> Store reader
    │
    ▼
Aggregate Metadata
    │
    ├─> Merge all bboxes → collection bbox
    ├─> Union all LODs → ["0", "1", "2", "3"]
    ├─> Union all types → ["Building", "TINRelief", ...]
    ├─> Union encodings → ["CityJSON", "FlatCityBuf"]
    ├─> Count stats → {min: 45, max: 5234, total: 125432}
    │
    ▼
Generate Collection
    │
    ├─> StacCollectionBuilder::new()
    ├─> .spatial_extent(merged_bbox)
    ├─> .aggregate_cityjson_metadata(&readers)
    ├─> .build()
    │
    ▼
Generate Items
    │
    ├─> For each reader:
    │   ├─> Build STAC Item
    │   ├─> Write items/building_001_item.json
    │   ├─> Write items/building_002_item.json
    │   └─> Write items/terrain_item.json
    │
    ▼
Write Collection
    │
    ├─> collection.json
    │
    ▼
Success Summary
    │
    └─> "✓ Generated 156 items
         Collection: ./stac_output/collection.json
         Items: ./stac_output/items/"
```

## Module Dependencies

```
main.rs
  │
  └─> cli::run()
        │
        ├─> cli::handle_item_command()
        │     │
        │     ├─> reader::get_reader()
        │     │     │
        │     │     └─> reader::cityjson::CityJSONReader
        │     │
        │     ├─> stac::StacItemBuilder
        │     │     │
        │     │     └─> metadata::* (BBox3D, CRS, etc.)
        │     │
        │     └─> serde_json::to_string_pretty()
        │
        ├─> cli::handle_collection_command()
        │     │
        │     ├─> traversal::find_files()
        │     │
        │     ├─> reader::get_reader() (multiple)
        │     │
        │     └─> stac::StacCollectionBuilder
        │
        └─> cli::handle_validate_command()
              │
              └─> stac::validate()
```

## Data Flow Diagram

```
┌─────────────┐
│ Input Files │
│             │
│ .json       │
│ .jsonl      │
│ .fcb        │
└──────┬──────┘
       │
       ▼
┌──────────────────┐
│  File Reader     │──┐
│                  │  │
│ Parse Structure  │  │
│ Load Metadata    │  │
└────────┬─────────┘  │
         │            │
         ▼            │
┌──────────────────┐  │
│   Metadata       │  │
│   Extraction     │  │
│                  │  │
│ • BBox          │  │ Multiple Files
│ • CRS           │  │ (Collection)
│ • LODs          │  │
│ • Types         │  │
│ • Attributes    │  │
└────────┬─────────┘  │
         │            │
         ▼            │
┌──────────────────┐  │
│   Aggregation    │◄─┘
│   (Optional)     │
│                  │
│ • Merge BBoxes   │
│ • Union Sets     │
│ • Statistics     │
└────────┬─────────┘
         │
         ▼
┌──────────────────┐
│   STAC Builder   │
│                  │
│ • Item Builder   │
│ • Collection     │
│   Builder        │
└────────┬─────────┘
         │
         ▼
┌──────────────────┐
│   STAC Objects   │
│                  │
│ • properties     │
│ • assets         │
│ • links          │
│ • summaries      │
└────────┬─────────┘
         │
         ▼
┌──────────────────┐
│  JSON Output     │
│                  │
│ item.json        │
│ collection.json  │
└──────────────────┘
```

## Error Handling Flow

```
┌─────────────┐
│ User Input  │
└──────┬──────┘
       │
       ▼
┌──────────────────────────────────┐
│ Validation Layer                 │
│                                  │
│ File exists?   ──No──> IoError  │
│ Format valid?  ──No──> UnsupportedFormat
│ Readable?      ──No──> PermissionError
└──────┬───────────────────────────┘
       │ Yes
       ▼
┌──────────────────────────────────┐
│ Processing Layer                 │
│                                  │
│ Parse error?   ──Yes──> JsonError│
│ Missing field? ──Yes──> MissingField
│ Invalid data?  ──Yes──> MetadataError
└──────┬───────────────────────────┘
       │ No errors
       ▼
┌──────────────────────────────────┐
│ Output Layer                     │
│                                  │
│ Write failed?  ──Yes──> IoError │
│ Invalid JSON?  ──Yes──> StacError│
└──────┬───────────────────────────┘
       │ Success
       ▼
┌──────────────┐
│   Success    │
│   Exit(0)    │
└──────────────┘
```

## Trait Implementation Pattern

```
┌─────────────────────────────────────────┐
│  CityModelMetadataReader Trait          │
│  (Abstract Interface)                   │
│                                         │
│  + bbox() -> Result<BBox3D>            │
│  + crs() -> Result<CRS>                │
│  + lods() -> Result<Vec<String>>       │
│  + city_object_types() -> ...          │
│  + ...                                 │
└────────────────┬────────────────────────┘
                 │
                 │ implements
                 │
    ┌────────────┼────────────┐
    │            │            │
    ▼            ▼            ▼
┌─────────┐  ┌─────────┐  ┌─────────┐
│CityJSON │  │CityJSON │  │FlatCity │
│ Reader  │  │SeqReader│  │BufReader│
└────┬────┘  └────┬────┘  └────┬────┘
     │            │            │
     │            │            │
     └────────────┴────────────┘
                  │
                  ▼
         Used polymorphically
                  │
                  ▼
      Box<dyn CityModelMetadataReader>
                  │
                  ▼
            STAC Builder
```

## Concurrency Model (Optional Parallel Processing)

```
┌──────────────────────────────────────┐
│ Main Thread                          │
│                                      │
│ 1. Scan directory                    │
│ 2. Collect file paths                │
│ 3. Create thread pool                │
└─────────────┬────────────────────────┘
              │
              ▼
┌──────────────────────────────────────┐
│ Rayon Thread Pool (if --parallel)    │
│                                      │
│  ┌─────────┐  ┌─────────┐  ┌─────────┐
│  │ Worker 1│  │ Worker 2│  │ Worker 3│
│  │         │  │         │  │         │
│  │ file_1  │  │ file_2  │  │ file_3  │
│  │ →reader │  │ →reader │  │ →reader │
│  │ →item   │  │ →item   │  │ →item   │
│  └────┬────┘  └────┬────┘  └────┬────┘
│       │            │            │
│       └────────────┴────────────┘
│                    │
└────────────────────┼──────────────────┘
                     │
                     ▼ Collect results
        ┌─────────────────────────┐
        │ Main Thread             │
        │                         │
        │ Aggregate metadata      │
        │ Build collection        │
        │ Write outputs           │
        └─────────────────────────┘
```

## Memory Management Strategy

### Streaming for Large Files

```
Small Files (< 100MB):
┌─────────────┐
│ Load entire │
│ file to     │
│ memory      │
└──────┬──────┘
       │
       ▼
    Process

Large Files (> 100MB):
┌─────────────┐
│ Stream      │
│ line-by-line│
│ (for .jsonl)│
└──────┬──────┘
       │
       ▼
  Accumulate
  metadata
  (small footprint)
```

### Lazy Loading

```
CityJSONReader {
    file_path: PathBuf,
    data: Option<Value>,  // Initially None
}

impl CityJSONReader {
    fn bbox(&mut self) -> Result<BBox3D> {
        self.ensure_loaded()?;  // Load only when needed
        // Extract bbox from loaded data
    }
}
```

## Extension Architecture

```
┌─────────────────────────────────────┐
│      STAC Core Schema               │
│                                     │
│  - type, id, bbox, geometry        │
│  - properties, assets, links       │
└──────────────┬──────────────────────┘
               │
               │ extends
               ▼
┌─────────────────────────────────────┐
│  CityJSON Extension (cj:)           │
│                                     │
│  - cj:encoding                     │
│  - cj:version                      │
│  - cj:city_objects                 │
│  - cj:lods                         │
│  - cj:co_types                     │
│  - cj:attributes                   │
│  - cj:transform                    │
│  - cj:metadata                     │
└─────────────────────────────────────┘
               │
               │ compatible with
               ▼
┌─────────────────────────────────────┐
│  Other STAC Extensions              │
│                                     │
│  - proj:epsg (Projection)          │
│  - file:size (File)                │
│  - processing:level (Processing)   │
└─────────────────────────────────────┘
```

## Testing Architecture

```
┌─────────────────────────────────────┐
│         Test Fixtures               │
│                                     │
│  tests/fixtures/                   │
│  ├── simple_building.json          │
│  ├── large_city.json               │
│  ├── features.jsonl                │
│  └── terrain.fcb                   │
└──────────────┬──────────────────────┘
               │
               ▼
┌─────────────────────────────────────┐
│         Unit Tests                  │
│                                     │
│  test_bbox_merge()                 │
│  test_crs_parsing()                │
│  test_lod_extraction()             │
│  test_stac_builder()               │
└──────────────┬──────────────────────┘
               │
               ▼
┌─────────────────────────────────────┐
│      Integration Tests              │
│                                     │
│  test_cityjson_to_item()           │
│  test_directory_to_collection()    │
│  test_mixed_formats()              │
└──────────────┬──────────────────────┘
               │
               ▼
┌─────────────────────────────────────┐
│         E2E Tests                   │
│                                     │
│  test_cli_item_command()           │
│  test_cli_collection_command()     │
│  test_error_handling()             │
└─────────────────────────────────────┘
```

This architecture provides:
- **Modularity**: Clear separation of concerns
- **Extensibility**: Easy to add new readers
- **Testability**: Each component can be tested independently
- **Performance**: Lazy loading and optional parallelization
- **Robustness**: Comprehensive error handling
