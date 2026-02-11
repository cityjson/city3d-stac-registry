# CityJSON-STAC Development Roadmap

## Context

This document outlines the development roadmap for adding three major features to cityjson-stac:
1. **YAML configuration** - Allow collection metadata to be specified via config file
2. **Remote access via HTTPS** - Enable reading files from remote servers using object_store abstraction
3. **CityGML support** - Add CityGML 2.0 and 3.0 format support

These features will expand the tool's capability to handle remote datasets, support XML-based 3D city models, and provide better metadata management for STAC collections.

**Implementation Order**: 3 → 1 → 2 (YAML config first for quick wins, then remote access, then CityGML)

**Key Decisions**:
- Remote files: Stream to memory (simpler implementation)
- CityGML: Streaming XML parser for all files (handles large files >1GB)
- STAC prefix: Break compatibility, use `city3d:` (clean break)

---

## Phase 1: YAML Configuration (Start Here - Quick Win)

### Goal
Allow users to specify collection metadata (title, license, providers, etc.) via a YAML config file instead of CLI arguments.

### Why Start Here?
- No async migration required
- No new parsing logic
- Purely additive feature
- Quick value to users

### Key Design Decisions

**Config Structure:**
- YAML file for human-friendly editing
- Optional - CLI arguments take precedence
- Can specify all collection-level metadata

**Schema Validation:**
- Use `schemars` to generate JSON Schema from config struct
- Optional validation via `jsonschema` crate (Phase 3.5)

### Implementation Steps

#### 1.1 Add YAML Dependencies
```toml
# Cargo.toml additions
serde_yaml = "0.9"
```

#### 1.2 Create Config Module
**New file:** `src/config/mod.rs`

```rust
use serde::{Deserialize, Serialize};

/// Collection configuration from YAML file
#[derive(Debug, Deserialize, Serialize, Default)]
pub struct CollectionConfigFile {
    /// Collection ID
    pub id: Option<String>,

    /// Collection title
    pub title: Option<String>,

    /// Collection description
    pub description: Option<String>,

    /// Data license (SPDX identifier)
    pub license: Option<String>,

    /// Keywords/tags
    pub keywords: Option<Vec<String>>,

    /// Providers (organizations that provided/manage data)
    pub providers: Option<Vec<ProviderConfig>>,

    /// Custom extent (overrides auto-detected)
    pub extent: Option<ExtentConfig>,

    /// Custom summaries (merged with auto-detected)
    pub summaries: Option<std::collections::HashMap<String, serde_json::Value>>,

    /// Links to add
    pub links: Option<Vec<LinkConfig>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ProviderConfig {
    pub name: String,
    pub url: Option<String>,
    pub roles: Option<Vec<String>>,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ExtentConfig {
    pub spatial: Option<SpatialExtentConfig>,
    pub temporal: Option<TemporalExtentConfig>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SpatialExtentConfig {
    pub bbox: Option<Vec<f64>>,
    pub crs: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TemporalExtentConfig {
    pub start: Option<String>,
    pub end: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct LinkConfig {
    pub rel: String,
    pub href: String,
    pub r#type: Option<String>,
    pub title: Option<String>,
}

impl CollectionConfigFile {
    /// Load config from YAML file
    pub fn from_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = serde_yaml::from_str(&content)
            .map_err(|e| CityJsonStacError::Other(format!("Invalid YAML: {}", e)))?;
        Ok(config)
    }

    /// Merge with CLI arguments (CLI takes precedence)
    pub fn merge_with_cli(self, cli_args: &CollectionCliArgs) -> Self {
        CollectionConfigFile {
            id: cli_args.id.clone().or(self.id),
            title: cli_args.title.clone().or(self.title),
            description: cli_args.description.clone().or(self.description),
            license: if cli_args.license != "proprietary" {
                Some(cli_args.license.clone())
            } else {
                self_license
            },
            keywords: self.keywords,
            providers: self.providers,
            extent: self.extent,
            summaries: self.summaries,
            links: self.links,
        }
    }
}

/// CLI arguments that can override config
pub struct CollectionCliArgs {
    pub id: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub license: String,
}
```

#### 1.3 Update CLI
**Modify:** `src/cli/mod.rs`

```rust
Collection {
    // ... existing fields ...

    /// YAML configuration file for collection metadata
    #[arg(short = 'C', long)]
    config: Option<PathBuf>,
}

fn handle_collection_command(config: CollectionConfig) -> Result<()> {
    // Load config file if provided
    let base_config = if let Some(config_path) = config.config {
        CollectionConfigFile::from_file(&config_path)?
    } else {
        CollectionConfigFile::default()
    };

    // Merge with CLI args
    let merged_config = base_config.merge_with_cli(&CollectionCliArgs {
        id: config.id,
        title: config.title,
        description: config.description,
        license: config.license,
    });

    // Build collection ID
    let collection_id = merged_config.id.unwrap_or_else(|| {
        config
            .directory
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("collection")
            .to_string()
    });

    let mut collection_builder = StacCollectionBuilder::new(&collection_id)
        .license(merged_config.license.unwrap_or_else(|| "proprietary".to_string()))
        .temporal_extent(Some(chrono::Utc::now()), None);

    // Apply config-based metadata
    if let Some(t) = merged_config.title {
        collection_builder = collection_builder.title(t);
    }

    if let Some(d) = merged_config.description {
        collection_builder = collection_builder.description(d);
    }

    if let Some(providers) = merged_config.providers {
        for provider in providers {
            collection_builder = collection_builder.provider(provider.into());
        }
    }

    if let Some(keywords) = merged_config.keywords {
        collection_builder = collection_builder.keywords(keywords);
    }

    // ... rest of collection building (aggregation, etc.)
}
```

#### 1.4 Provider Conversion
**Modify:** `src/stac/models.rs` or `src/config/mod.rs`

```rust
impl From<ProviderConfig> for crate::stac::models::Provider {
    fn from(config: ProviderConfig) -> Self {
        crate::stac::models::Provider {
            name: config.name,
            description: config.description,
            url: config.url,
            roles: config.roles.unwrap_or_default(),
            // ... other fields set to defaults
        }
    }
}
```

### Files to Modify
- `Cargo.toml` - Add serde_yaml dependency
- `src/cli/mod.rs` - Add `--config` option, merge logic
- `src/stac/models.rs` - Add Provider conversion
- `src/main.rs` - Export config module

### Files to Create
- `src/config/mod.rs` - Config structures and parsing
- `examples/collection-config.yaml` - Example configuration file
- `tests/config_tests.rs` - Config parsing tests

### Example Config File
```yaml
# examples/collection-config.yaml
id: rotterdam-3d-city-model
title: Rotterdam 3D City Model 2023
description: |
  Complete 3D city model of Rotterdam containing buildings,
  roads, and terrain in multiple Levels of Detail.
license: CC-BY-4.0

keywords:
  - 3d city model
  - Rotterdam
  - LoD2
  - CityJSON

providers:
  - name: City of Rotterdam
    url: https://rotterdam.nl
    roles:
      - producer
      - licensor

extent:
  spatial:
    bbox: [4.42, 51.88, 4.6, 51.98]
    crs: EPSG:7415
  temporal:
    start: "2023-01-01T00:00:00Z"
    end: null

summaries:
  custom:field: "custom value"

links:
  - rel: license
    href: https://creativecommons.org/licenses/by/4.0/
    type: text/html
```

### Testing
- Config parsing tests
- Merge logic tests (config + CLI)
- Invalid YAML handling
- Example file validation

---

## Phase 2: Remote Access via HTTPS

### Goal
Enable readers to access files from remote servers (HTTPS, S3, Azure Blob, etc.) using Apache arrow-rs-object_store for abstraction.

### Key Design Decisions

**Async Migration Strategy:**
- The codebase is currently entirely synchronous. Remote access requires async I/O.
- Introduce `tokio` as the async runtime.
- Create a dual-mode reader architecture: sync for local files, async for remote.
- Stream to memory (simpler implementation as per user decision)

**Object Store Integration:**
- Use `object_store` crate for abstraction over different storage backends.
- Support HTTP/HTTPS (via `http` feature), local files (existing), S3, Azure.
- Implement remote download to memory buffer.

### Implementation Steps

#### 2.1 Add Dependencies
```toml
# Cargo.toml additions
tokio = { version = "1.40", features = ["fs", "rt-multi-thread"] }
object_store = { version = "0.11", features = ["http", "aws", "azure"] }
reqwest = { version = "0.12", features = ["rustls-tls"] }
```

#### 2.2 Create Remote Access Module
**New file:** `src/remote/mod.rs`

```rust
use object_store::ObjectStore;
use reqwest::Client;

/// Remote location types
pub enum RemoteLocation {
    Http(String),
    Https(String),
    S3 { bucket: String, key: String },
    Azure { container: String, path: String },
}

impl RemoteLocation {
    pub fn parse(input: &str) -> Result<Self> {
        if input.starts_with("https://") {
            Ok(RemoteLocation::Https(input.to_string()))
        } else if input.starts_with("http://") {
            Ok(RemoteLocation::Http(input.to_string()))
        } else if input.starts_with("s3://") {
            // Parse s3://bucket/key
            let rest = input.strip_prefix("s3://").unwrap();
            let parts: Vec<&str> = rest.splitn(2, '/').collect();
            if parts.len() == 2 {
                Ok(RemoteLocation::S3 {
                    bucket: parts[0].to_string(),
                    key: parts[1].to_string(),
                })
            } else {
                Err(CityJsonStacError::Other("Invalid S3 URL".to_string()))
            }
        } else {
            Err(CityJsonStacError::Other("Unsupported remote location".to_string()))
        }
    }
}

/// Download remote content to memory
pub async fn fetch_to_bytes(location: &RemoteLocation) -> Result<bytes::Bytes> {
    match location {
        RemoteLocation::Http(url) | RemoteLocation::Https(url) => {
            let client = Client::new();
            let response = client.get(url).send().await
                .map_err(|e| CityJsonStacError::Other(format!("HTTP error: {}", e)))?;

            if !response.status().is_success() {
                return Err(CityJsonStacError::Other(
                    format!("HTTP status: {}", response.status())
                ));
            }

            Ok(response.bytes().await
                .map_err(|e| CityJsonStacError::Other(format!("HTTP read error: {}", e)))?)
        }
        _ => Err(CityJsonStacError::Other("Not yet implemented".to_string()))
    }
}
```

#### 2.3 Refactor Reader Factory
**Modify:** `src/reader/mod.rs`

```rust
/// Input that can be a local path or remote URL
pub enum InputSource {
    Local(PathBuf),
    Remote(String),
}

impl InputSource {
    pub fn from_str(input: &str) -> Result<Self> {
        if input.starts_with("http://") || input.starts_with("https://") ||
           input.starts_with("s3://") || input.starts_with("az://") {
            Ok(InputSource::Remote(input.to_string()))
        } else {
            Ok(InputSource::Local(PathBuf::from(input)))
        }
    }
}

/// Factory that accepts both local paths and URLs
pub fn get_reader_from_source(source: &InputSource) -> Result<Box<dyn CityModelMetadataReader>> {
    match source {
        InputSource::Local(path) => get_reader(path),
        InputSource::Remote(url) => {
            // Detect format from URL extension
            let extension = extract_extension_from_url(url)?;

            match extension.to_lowercase().as_str() {
                "json" => Ok(Box::new(RemoteCityJSONReader::from_url(url)?)),
                "jsonl" | "cjseq" => Ok(Box::new(RemoteCityJSONSeqReader::from_url(url)?)),
                _ => Err(CityJsonStacError::UnsupportedFormat(format!(
                    "Unknown extension: {}", extension
                )))
            }
        }
    }
}

fn extract_extension_from_url(url: &str) -> Result<String> {
    // Extract extension from URL path
    url.split('/')
        .last()
        .and_then(|s| s.split('?').next()) // Remove query string
        .and_then(|s| s.rsplit('.').next())
        .ok_or_else(|| CityJsonStacError::Other("No extension in URL".to_string()))
        .map(|s| s.to_string())
}
```

#### 2.4 Implement Remote Readers
**New files:** `src/reader/remote_cityjson.rs`, `src/reader/remote_cjseq.rs`

```rust
use crate::remote::fetch_to_bytes;

pub struct RemoteCityJSONReader {
    url: String,
    data: RwLock<Option<serde_json::Value>>,
}

impl RemoteCityJSONReader {
    pub async fn from_url(url: &str) -> Result<Self> {
        let bytes = fetch_to_bytes(&RemoteLocation::Https(url.to_string())).await?;
        // Parse JSON from bytes
        let json: serde_json::Value = serde_json::from_slice(&bytes)
            .map_err(|e| CityJsonStacError::JsonError(e))?;

        Ok(Self {
            url: url.to_string(),
            data: RwLock::new(Some(json)),
        })
    }
}

impl CityModelMetadataReader for RemoteCityJSONReader {
    fn file_path(&self) -> &Path {
        // Return a virtual path for remote URLs
        Path::new(&self.url)
    }

    fn encoding(&self) -> &'static str { "CityJSON" }

    // ... other trait methods similar to CityJSONReader
}
```

#### 2.5 Update CLI for URL Inputs
**Modify:** `src/cli/mod.rs`

```rust
Item {
    /// Input file path or URL
    #[arg(value_parser = parse_input_source)]
    input: InputSource,

    // ... other fields
}

Collection {
    /// Directory to scan (local only - remote scanning not supported)
    directory: PathBuf,  // Keep as PathBuf for now

    // ... other fields
}

fn parse_input_source(s: &str) -> Result<InputSource> {
    InputSource::from_str(s)
}
```

**Note:** Collection command still requires local directory for scanning. Remote file support for collections would require listing capability from object stores (future enhancement).

#### 2.6 Make Main Async
**Modify:** `src/main.rs`

```rust
#[tokio::main]
async fn main() -> Result<()> {
    cli::run().await
}
```

**Modify:** `src/cli/mod.rs`

```rust
pub async fn run() -> Result<()> {
    let cli = Cli::parse();

    // ... logging setup

    match cli.command {
        Commands::Item { .. } => handle_item_command_async(...).await,
        Commands::Collection { .. } => handle_collection_command(...),  // Still sync
        Commands::UpdateCollection { .. } => handle_update_collection_command(...),  // Still sync
    }
}
```

### Files to Modify
- `Cargo.toml` - Add async dependencies
- `src/main.rs` - Add tokio main
- `src/cli/mod.rs` - URL input support, async handler for item
- `src/reader/mod.rs` - Update factory for URL detection
- `src/error.rs` - Add network-related errors

### Files to Create
- `src/remote/mod.rs` - Remote access abstraction
- `src/reader/remote_cityjson.rs` - HTTP-enabled CityJSON reader
- `src/reader/remote_cjseq.rs` - HTTP-enabled CityJSONSeq reader

### Testing
- Mock HTTP server for testing
- Test with real HTTPS URLs
- Verify error handling for network failures
- Test timeout behavior

---

## Phase 3: CityGML Support

### Goal
Add support for CityGML 2.0 and 3.0 formats using `quick-xml` for parsing.

### Key Design Decisions

**XML Parsing Strategy:**
- Use `quick-xml` with `serde` for deserialization
- CityGML is complex and modular - need to handle multiple namespaces
- **Streaming parser for all files** (as per user decision - handles files >1GB)

**Metadata Extraction:**
- Map CityGML concepts to `CityModelMetadataReader` trait methods:
  - `cityObjectMember` → `city_object_types()`
  - `boundedBy` → `bbox()`
  - `gml:id` attributes for object counting
  - `lodXGeometry` properties → `lods()`

**STAC Extension Prefix Change:**
- Current implementation uses `cj:` (CityJSON-specific)
- STAC extension spec uses `city3d:` (format-agnostic)
- **Break compatibility** - use `city3d:` only (as per user decision)

### Implementation Steps

#### 3.1 Add XML Dependencies
```toml
# Cargo.toml additions
quick-xml = { version = "0.37", features = ["serialize", "async-tokio"] }
```

#### 3.2 Create Streaming CityGML Reader
**New file:** `src/reader/citygml.rs`

```rust
use quick_xml::de::Deserializer;
use quick_xml::events::Event;
use std::io::BufReader;

pub struct CityGMLReader {
    file_path: PathBuf,
    metadata: RwLock<Option<CityGMLMetadata>>,
}

struct CityGMLMetadata {
    version: String,  // "2.0" or "3.0"
    city_object_types: BTreeSet<String>,
    bbox: Option<BBox3D>,
    crs: Option<CRS>,
    lods: BTreeSet<String>,
    city_object_count: usize,
    attributes: Vec<AttributeDefinition>,
    extensions: Vec<String>,
    has_semantic_surfaces: bool,
    has_textures: bool,
    has_materials: bool,
}

impl CityGMLReader {
    /// Parse CityGML using streaming parser (for large files)
    pub fn new(file_path: &Path) -> Result<Self> {
        let file = std::fs::File::open(file_path)?;
        let reader = BufReader::new(file);

        let mut metadata = CityGMLMetadata {
            version: Self::detect_version(reader)?
            // ... initialize other fields
        };

        // Stream through file to extract metadata
        Self::stream_parse_metadata(file_path, &mut metadata)?;

        Ok(Self {
            file_path: file_path.to_path_buf(),
            metadata: RwLock::new(Some(metadata)),
        })
    }

    fn detect_version(reader: BufReader<File>) -> Result<String> {
        // Read first few KB to find CityGML version
        // Look for xmlns="http://www.opengis.net/citygml/2.0" or /3.0
    }

    fn stream_parse_metadata(path: &Path, metadata: &mut CityGMLMetadata) -> Result<()> {
        // Use streaming XML parser to avoid loading entire file
        // Count city objects, collect types, extract bbox, etc.
        let reader = std::fs::File::open(path)?;
        let reader = BufReader::new(reader);
        let mut parser = quick_xml::Reader::from_reader(reader);

        let mut current_element = Vec::new();
        let mut in_city_object = false;

        loop {
            match parser.read_event_into(&mut current_element) {
                Ok(Event::Start(ref e)) => {
                    match e.name().as_ref() {
                        b"core:CityObject" | b"cityObjectMember" => {
                            in_city_object = true;
                            metadata.city_object_count += 1;
                        }
                        b"bldg:Building" => {
                            metadata.city_object_types.insert("Building".to_string());
                        }
                        b"tran:Road" => {
                            metadata.city_object_types.insert("Road".to_string());
                        }
                        b"lodXMultiSurface" | b"lodXSolid" => {
                            // Extract LOD number
                        }
                        // ... handle other element types
                    }
                }
                Ok(Event::End(_)) => {
                    in_city_object = false;
                }
                Ok(Event::Eof) => break,
                Err(e) => return Err(CityJsonStacError::Other(format!("XML error: {}", e))),
                _ => {}
            }
        }

        Ok(())
    }
}

impl CityModelMetadataReader for CityGMLReader {
    fn encoding(&self) -> &'static str { "CityGML" }

    fn version(&self) -> Result<String> {
        self.data.read().unwrap().as_ref()
            .map(|m| m.version.clone())
            .ok_or_else(|| CityJsonStacError::MetadataError("Version not found".to_string()))
    }

    fn bbox(&self) -> Result<BBox3D> {
        self.data.read().unwrap().as_ref()
            .and_then(|m| m.bbox.clone())
            .ok_or_else(|| CityJsonStacError::MetadataError("BBox not found".to_string()))
    }

    fn crs(&self) -> Result<CRS> {
        self.data.read().unwrap().as_ref()
            .and_then(|m| m.crs.clone())
            .ok_or_else(|| CityJsonStacError::MetadataError("CRS not found".to_string()))
    }

    fn lods(&self) -> Result<Vec<String>> {
        self.data.read().unwrap().as_ref()
            .map(|m| m.lods.iter().cloned().collect())
            .ok_or_else(|| CityJsonStacError::MetadataError("LODs not found".to_string()))
    }

    fn city_object_types(&self) -> Result<Vec<String>> {
        self.data.read().unwrap().as_ref()
            .map(|m| m.city_object_types.iter().cloned().collect())
            .ok_or_else(|| CityJsonStacError::MetadataError("Types not found".to_string()))
    }

    fn city_object_count(&self) -> Result<usize> {
        self.data.read().unwrap().as_ref()
            .map(|m| m.city_object_count)
            .ok_or_else(|| CityJsonStacError::MetadataError("Count not found".to_string()))
    }

    fn attributes(&self) -> Result<Vec<AttributeDefinition>> {
        self.data.read().unwrap().as_ref()
            .map(|m| m.attributes.clone())
            .ok_or_else(|| CityJsonStacError::MetadataError("Attributes not found".to_string()))
    }

    fn file_path(&self) -> &Path {
        &self.file_path
    }

    fn transform(&self) -> Result<Option<Transform>> {
        Ok(None)  // CityGML doesn't use vertex compression
    }

    fn metadata(&self) -> Result<Option<serde_json::Value>> {
        Ok(None)
    }

    fn extensions(&self) -> Result<Vec<String>> {
        self.data.read().unwrap().as_ref()
            .map(|m| m.extensions.clone())
            .ok_or_else(|| CityJsonStacError::MetadataError("Extensions not found".to_string()))
    }
}
```

#### 3.3 Update Factory
**Modify:** `src/reader/mod.rs`

```rust
pub fn get_reader(file_path: &Path) -> Result<Box<dyn CityModelMetadataReader>> {
    let extension = file_path
        .extension()
        .and_then(|e| e.to_str())
        .ok_or_else(|| {
            CityJsonStacError::UnsupportedFormat("No file extension found".to_string())
        })?;

    match extension.to_lowercase().as_str() {
        "gml" | "xml" => {
            // Check if CityGML
            if is_citygml(file_path)? {
                Ok(Box::new(CityGMLReader::new(file_path)?))
            } else {
                Err(CityJsonStacError::UnsupportedFormat(
                    "File is not a CityGML file".to_string(),
                ))
            }
        }
        "json" => {
            // ... existing CityJSON logic
        }
        "jsonl" | "cjseq" => {
            // ... existing CityJSONSeq logic
        }
        "fcb" => {
            // ... existing FlatCityBuf logic
        }
        _ => Err(CityJsonStacError::UnsupportedFormat(format!(
            "Unknown extension: {}",
            extension
        ))),
    }
}

fn is_citygml(file_path: &Path) -> Result<bool> {
    use std::io::{BufRead, BufReader};
    let file = std::fs::File::open(file_path)?;
    let reader = BufReader::new(file);

    // Read first few lines and look for CityGML namespace
    for line in reader.lines().take(20) {
        let line = line?;
        if line.contains("www.opengis.net/citygml") {
            return Ok(true);
        }
    }

    Ok(false)
}
```

#### 3.4 Update STAC Extension Prefix (Breaking Change)
**Modify:** `src/stac/item.rs`, `src/stac/collection.rs`

- Change `cj:` → `city3d:` for all extension properties
- Update schema URL reference

```rust
// In item.rs - cityjson_metadata():
// Old:
self.properties.insert("cj:encoding".to_string(), json!(reader.encoding()));
self.properties.insert("cj:version".to_string(), json!(reader.version()?));

// New:
self.properties.insert("city3d:encoding".to_string(), json!(reader.encoding()));
self.properties.insert("city3d:version".to_string(), json!(reader.version()?));
```

```rust
// In collection.rs - aggregate_cityjson_metadata():
// Old:
self.summaries.insert("cj:encoding".to_string(), ...);
self.summaries.insert("cj:lods".to_string(), ...);

// New:
self.summaries.insert("city3d:encoding".to_string(), ...);
self.summaries.insert("city3d:lods".to_string(), ...);
```

#### 3.5 Update STAC Extension URL
**Modify:** `src/stac/models.rs`

```rust
// In StacCollection build():
stac_extensions: vec![
    "https://stac-extensions.github.io/3d-city-models/v0.1.0/schema.json".to_string(),
    "https://stac-extensions.github.io/projection/v1.1.0/schema.json".to_string(),
],
```

### Files to Modify
- `Cargo.toml` - Add XML dependencies
- `src/reader/mod.rs` - Add .gml/.xml extension handling
- `src/stac/item.rs` - Update property prefix to `city3d:`
- `src/stac/collection.rs` - Update summary prefix to `city3d:`
- `src/stac/models.rs` - Update extension URL

### Files to Create
- `src/reader/citygml.rs` - CityGML reader implementation
- `tests/fixtures/sample_citygml_2.gml` - Test fixture
- `tests/fixtures/sample_citygml_3.gml` - Test fixture

### Testing
- Sample CityGML 2.0 and 3.0 test files
- Verify LOD extraction from different CityGML modules
- Test streaming parser with large files (>100MB)
- Verify XML namespace handling
- Test attribute extraction from GenericCityObject

---

## Milestones

### Phase 1: YAML Configuration

#### Milestone 1.0: Basic YAML Config
- [ ] Add serde_yaml dependency
- [ ] Create config module with structures
- [ ] Implement config file loading
- [ ] Add `--config` CLI option
- [ ] Merge logic (config + CLI args)
- [ ] Provider configuration support
- [ ] Example config file
- [ ] Tests for config parsing

**Estimated complexity:** Low (no async, no new parsing)

#### Milestone 1.5: Advanced YAML Config
- [ ] Custom extent configuration
- [ ] Custom summaries configuration
- [ ] Custom links configuration
- [ ] Config validation errors
- [ ] Documentation updates

### Phase 2: Remote Access

#### Milestone 2.0: HTTP/HTTPS Support
- [ ] Add tokio and reqwest dependencies
- [ ] Create remote module with fetch function
- [ ] Implement `RemoteCityJSONReader`
- [ ] Implement `RemoteCityJSONSeqReader`
- [ ] Update factory for URL detection
- [ ] CLI support for URL inputs (item command)
- [ ] Make main async
- [ ] Tests with mock HTTP server

**Estimated complexity:** High (requires async migration)

#### Milestone 2.5: Cloud Storage Support
- [ ] S3 support (`s3://` URLs)
- [ ] Azure Blob Storage support (`az://` URLs)
- [ ] Progress indicators for downloads
- [ ] Timeout handling
- [ ] Retry logic for failed downloads

### Phase 3: CityGML Support

#### Milestone 3.0: CityGML Basic Support
- [ ] Add quick-xml dependency
- [ ] Create `CityGMLReader` with streaming parser
- [ ] Implement version detection (2.0 vs 3.0)
- [ ] Implement `city_object_types()` extraction
- [ ] Implement `bbox()` extraction
- [ ] Implement `crs()` extraction
- [ ] Update factory for .gml/.xml files
- [ ] Tests with sample CityGML files

#### Milestone 3.5: CityGML Complete
- [ ] LOD extraction for CityGML
- [ ] Attribute extraction from CityGML
- [ ] Handle CityGML 2.0 vs 3.0 differences
- [ ] Extension (ADE) detection
- [ ] Semantic surface detection
- [ ] Texture/material detection
- [ ] Large file streaming tests

#### Milestone 3.6: STAC Extension Prefix Update (Breaking Change)
- [ ] Update all `cj:` to `city3d:` in item builder
- [ ] Update all `cj:` to `city3d:` in collection builder
- [ ] Update STAC extension schema URL
- [ ] Update documentation
- [ ] Update STAC_EXTENSION.md

---

## Dependencies Summary

### New Dependencies Required
| Crate | Version | Features | Phase |
|--------|----------|-----------|-------|
| serde_yaml | 0.9+ | - | 1 |
| tokio | 1.40+ | fs, rt-multi-thread | 2 |
| reqwest | 0.12+ | rustls-tls | 2 |
| quick-xml | 0.37+ | serialize, async-tokio | 3 |
| object_store | 0.11+ | http, aws, azure | 2.5 (optional) |

---

## Testing Strategy

### Unit Tests
- Config parsing tests (Phase 1)
- Remote download mocking (Phase 2)
- XML parsing tests (Phase 3)

### Integration Tests
- Config file + CLI interaction (Phase 1)
- End-to-end with real HTTP endpoint (Phase 2)
- CityGML file processing (Phase 3)

### Test Fixtures Needed
- Config file examples (Phase 1)
- Sample CityGML 2.0 file (Phase 3)
- Sample CityGML 3.0 file (Phase 3)

---

## Breaking Changes

1. **STAC Extension Prefix:** `cj:` → `city3d:` (Phase 3.6)
   - Existing STAC outputs will use old prefix
   - Users will need to regenerate collections
   - This is a clean break as per user decision

2. **Async Runtime:** CLI will become async (Phase 2)
   - Should be transparent to users
   - Main function signature changes

3. **New Network Errors:** Remote access may fail (Phase 2)
   - New error types for HTTP failures
   - Timeout errors

---

## Post-Implementation Documentation

After implementing all phases, update:

1. **README.md** - New features and usage examples
2. **CLAUDE.md** - Update project instructions
3. **STAC_EXTENSION.md** - Confirm prefix is `city3d:`
4. **CHANGELOG.md** - Document breaking changes
5. **examples/** - Add config file examples

---

## Verification Steps

After each phase, verify:

### Phase 1 (YAML Config)
```bash
# Test config file loading
cjstac collection ./data --config examples/collection-config.yaml -o ./stac_output

# Verify config values are used
grep '"Rotterdam 3D City Model 2023"' ./stac_output/collection.json
```

### Phase 2 (Remote Access)
```bash
# Test URL input
cjstac item https://example.com/data/building.city.json -o ./item.json

# Verify item is created
cat ./item.json | jq '.properties["city3d:encoding"]'
```

### Phase 3 (CityGML)
```bash
# Test CityGML support
cjstac item tests/fixtures/sample_citygml_2.gml -o ./item.json

# Verify CityGML metadata
cat ./item.json | jq '.properties["city3d:encoding"]'  # Should be "CityGML"
```
