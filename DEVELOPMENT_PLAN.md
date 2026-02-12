# CityJSON-STAC Development Roadmap

## Status Update (Feb 2025)

**Completed in commit #13:**
- ✅ STAC extension prefix changed from `cj:` to `city3d:`
- ✅ CityGML encoding added to supported formats
- ✅ STAC_EXTENSION.md updated to match official spec
- ✅ Schema files updated

**Remaining Work:**
- ⏳ YAML Configuration for collections
- ⏳ Remote access via HTTPS (async migration)
- ⏳ CityGML reader implementation (XML parsing)

---

## Context

This document outlines the remaining development roadmap for three features:
1. **YAML configuration** - Allow collection metadata to be specified via config file
2. **Remote access via HTTPS** - Enable reading files from remote servers
3. **CityGML support** - Add CityGML 2.0 and 3.0 format support (reader only, encoding exists)

These features will expand the tool's capability to handle remote datasets, support XML-based 3D city models, and provide better metadata management for STAC collections.

**Implementation Order**: 1 → 2 → 3 (YAML config first for quick wins, then remote access, then CityGML reader)

**Key Decisions**:
- Remote files: Stream to memory (simpler implementation)
- CityGML: Streaming XML parser for all files (handles large files >1GB)
- STAC prefix: Already updated to `city3d:` ✅

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
    pub links: Option<Vec<LinkConfig>>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ProviderConfig {
    pub name: String,
    pub url: Option<String>>,
    pub roles: Option<Vec<String>>,
    pub description: Option<String>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ExtentConfig {
    pub spatial: Option<SpatialExtentConfig>,
    pub temporal: Option<TemporalExtentConfig>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SpatialExtentConfig {
    pub bbox: Option<Vec<f64>>>,
    pub crs: Option<String>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TemporalExtentConfig {
    pub start: Option<String>>,
    pub end: Option<String>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct LinkConfig {
    pub rel: String,
    pub href: String,
    pub r#type: Option<String>>,
    pub title: Option<String>>,
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
                self.license
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
    pub id: Option<String>>,
    pub title: Option<String>>,
    pub description: Option<String>>,
    pub license: String,
}
```

#### 1.3 Update CLI
**Modify:** `src/cli/mod.rs`

Add `--config` option to Collection command:

```rust
Collection {
    // ... existing fields ...

    /// YAML configuration file for collection metadata
    #[arg(short = 'C', long)]
    config: Option<PathBuf>,
}
```

Update `handle_collection_command()` to load and merge config:

```rust
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
**Modify:** `src/stac/models.rs`

Add `From<ProviderConfig>` implementation:

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
Enable readers to access files from remote servers (HTTPS, S3, Azure Blob, etc.) using object_store abstraction.

### Key Design Decisions

**Async Migration Strategy:**
- The codebase is currently entirely synchronous. Remote access requires async I/O.
- Introduce `tokio` as async runtime.
- Create a dual-mode reader architecture: sync for local files, async for remote.
- Stream to memory (simpler implementation as per user decision)

### Implementation Steps

#### 2.1 Add Dependencies
```toml
# Cargo.toml additions
tokio = { version = "1.40", features = ["fs", "rt-multi-thread"] }
object_store = { version = "0.11", features = ["http", "aws", "azure"] }
reqwest = { version = "0.12", features = ["rustls-tls"] }
bytes = "1.8"
```

#### 2.2 Create Remote Access Module
**New file:** `src/remote/mod.rs`

```rust
use reqwest::Client;
use bytes::Bytes;

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
                "gml" => Ok(Box::new(RemoteCityGMLReader::from_url(url)?)),
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

    fn version(&self) -> Result<String> {
        self.data.read().unwrap().as_ref()
            .and_then(|v| v.get("version"))
            .and_then(|v| v.as_str())
            .map(String::from)
            .ok_or_else(|| CityJsonStacError::MetadataError("Version not found".to_string()))
    }

    fn bbox(&self) -> Result<BBox3D> {
        self.data.read().unwrap().as_ref()
            .and_then(|v| v.get("metadata"))
            .and_then(|m| m.get("geographicalExtent"))
            .and_then(|e| e.as_array())
            .and_then(|a| a.as_array())
            .and_then(|a| a.first())
            .and_then(|b| b.as_array())
            .ok_or_else(|| CityJsonStacError::MetadataError("BBox not found".to_string()))
            .map(|bbox| {
                // Parse bbox array [xmin, ymin, zmin, xmax, ymax, zmax]
                let coords = bbox.as_array().unwrap();
                BBox3D::new(
                    coords[0].as_f64().unwrap(),
                    coords[1].as_f64().unwrap(),
                    coords[2].as_f64().unwrap(),
                    coords[3].as_f64().unwrap(),
                    coords[4].as_f64().unwrap(),
                    coords[5].as_f64().unwrap(),
                )
            })
    }

    fn crs(&self) -> Result<CRS> {
        self.data.read().unwrap().as_ref()
            .and_then(|v| v.get("metadata"))
            .and_then(|m| m.get("referenceSystem"))
            .and_then(|rs| rs.as_str())
            .map(String::from)
            .ok_or_else(|| CityJsonStacError::MetadataError("CRS not found".to_string()))
            .map(|crs_str| {
                // Parse CRS from string like "https://www.opengis.net/def/crs/EPSG/0/7415"
                if crs_str.contains("/EPSG/0/") {
                    let epsg_code: u32 = crs_str.split('/').last()
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(0);
                    CRS::from_epsg(epsg_code)
                } else {
                    CRS::from_wkt2(crs_str.clone())
                }
            })
    }

    fn lods(&self) -> Result<Vec<String>> {
        self.data.read().unwrap().as_ref()
            .and_then(|v| v.get("CityObjects"))
            .and_then(|co| co.as_object())
            .and_then(|obj| {
                // Collect LODs from all CityObjects
                let mut lods = std::collections::BTreeSet::new();
                for (_key, value) in obj.iter() {
                    if let Some(obj) = value.as_object() {
                        if let Some(geom) = obj.get("geometry") {
                            if let Some(geoms) = geom.as_array() {
                                for g in geoms.iter() {
                                    if let Some(lod) = g.get("lod") {
                                        if let Some(lod_str) = lod.as_str() {
                                            lods.insert(lod_str.to_string());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                if !lods.is_empty() {
                    Ok(lods.into_iter().collect())
                } else {
                    Err(CityJsonStacError::MetadataError("No LODs found".to_string()))
                }
            })
            .ok_or_else(|| CityJsonStacError::MetadataError("CityObjects not found".to_string()))
    }

    fn city_object_types(&self) -> Result<Vec<String>> {
        self.data.read().unwrap().as_ref()
            .and_then(|v| v.get("CityObjects"))
            .and_then(|co| co.as_object())
            .map(|obj| {
                let mut types = std::collections::BTreeSet::new();
                for (key, _value) in obj.iter() {
                    // Extract type from key like "id1"
                    // CityJSON types can be standard or prefixed with "+"
                    let type_name = key.strip_prefix("id").unwrap_or(key);
                    types.insert(type_name.to_string());
                }
                types.into_iter().collect()
            })
            .ok_or_else(|| CityJsonStacError::MetadataError("CityObjects not found".to_string()))
    }

    fn city_object_count(&self) -> Result<usize> {
        self.data.read().unwrap().as_ref()
            .and_then(|v| v.get("CityObjects"))
            .and_then(|co| co.as_object())
            .map(|obj| obj.len())
            .ok_or_else(|| CityJsonStacError::MetadataError("CityObjects not found".to_string()))
    }

    fn attributes(&self) -> Result<Vec<AttributeDefinition>> {
        self.data.read().unwrap().as_ref()
            .and_then(|v| v.get("CityObjects"))
            .and_then(|co| {
                // Extract attributes from +GenericCityObject or type-specific objects
                co.as_object()
                    .and_then(|obj| {
                        // Look for attributes in first city object
                        obj.values().next()
                            .and_then(|val| val.as_object())
                            .and_then(|o| o.get("attributes"))
                            .and_then(|attrs| attrs.as_array())
                            .map(|arr| {
                                arr.iter().filter_map(|a| a.as_object()).map(|attr| {
                                    AttributeDefinition {
                                        name: attr.get("name").and_then(|v| v.as_str()).map(String::from).unwrap(),
                                        attr_type: AttributeType::String,
                                        description: attr.get("description").and_then(|v| v.as_str()).map(String::from),
                                        required: attr.get("required").and_then(|v| v.as_bool()),
                                    }
                                }).collect()
                            })
                    })
            })
            .unwrap_or(Ok(Vec::new()))
    }

    fn file_path(&self) -> &Path { &self.file_path }

    fn transform(&self) -> Result<Option<Transform>> {
        self.data.read().unwrap().as_ref()
            .and_then(|v| v.get("transform"))
            .and_then(|t| serde_json::from_value(t.clone()).ok())
            .ok_or_else(|| Ok(None))
    }

    fn metadata(&self) -> Result<Option<serde_json::Value>> {
        Ok(self.data.read().unwrap().as_ref().and_then(|v| v.get("metadata").cloned()))
    }

    fn extensions(&self) -> Result<Vec<String>> {
        self.data.read().unwrap().as_ref()
            .and_then(|v| v.get("extensions"))
            .and_then(|e| e.as_array())
            .map(|arr| {
                arr.iter().filter_map(|v| v.as_str()).map(String::from).collect()
            })
            .unwrap_or(Ok(Vec::new()))
    }
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

## Phase 3: CityGML Reader Implementation

### Goal
Add CityGML 2.0 and 3.0 format reader using `quick-xml` for parsing.

**Note:** The `city3d:encoding` property and "CityGML" value were already added in commit #13. This phase focuses on implementing the actual XML reader.

### Key Design Decisions

**XML Parsing Strategy:**
- Use `quick-xml` for parsing
- CityGML is complex and modular - need to handle multiple namespaces
- **Streaming parser for all files** (as per user decision - handles files >1GB)

**Metadata Extraction:**
- Map CityGML concepts to `CityModelMetadataReader` trait methods:
  - `cityObjectMember` → `city_object_types()`
  - `boundedBy` → `bbox()`
  - `gml:id` attributes for object counting
  - `lodXGeometry` properties → `lods()`
  - XML attributes for materials, textures, semantic surfaces

### Implementation Steps

#### 3.1 Add XML Dependencies
```toml
# Cargo.toml additions
quick-xml = { version = "0.37", features = ["serialize"] }
```

#### 3.2 Create Streaming CityGML Reader
**New file:** `src/reader/citygml.rs`

```rust
use quick_xml::events::Event;
use std::io::BufReader;
use crate::metadata::{BBox3D, CRS, AttributeDefinition};

pub struct CityGMLReader {
    file_path: PathBuf,
    metadata: RwLock<Option<CityGMLMetadata>>,
}

struct CityGMLMetadata {
    version: String,  // "2.0" or "3.0"
    city_object_types: std::collections::BTreeSet<String>,
    bbox: Option<BBox3D>,
    crs: Option<CRS>,
    lods: std::collections::BTreeSet<String>,
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
            version: Self::detect_version(&mut reader.lock())?,
            city_object_types: std::collections::BTreeSet::new(),
            bbox: None,
            crs: None,
            lods: std::collections::BTreeSet::new(),
            city_object_count: 0,
            attributes: Vec::new(),
            extensions: Vec::new(),
            has_semantic_surfaces: false,
            has_textures: false,
            has_materials: false,
        };

        // Stream through file to extract metadata
        Self::stream_parse_metadata(file_path, &mut metadata)?;

        Ok(Self {
            file_path: file_path.to_path_buf(),
            metadata: RwLock::new(Some(metadata)),
        })
    }

    fn detect_version(reader: &mut std::io::BufReader<std::fs::File>>) -> Result<String> {
        // Read first few KB to find CityGML version
        // Look for xmlns="http://www.opengis.net/citygml/2.0" or /3.0
        // Reset and read again
        use std::io::{BufRead, Seek};
        reader.seek(std::io::SeekFrom::Start(0))?;

        let mut buffer = Vec::new();
        let bytes_read = reader.read_buf(&mut buffer).take(4096)?; // Read first 4KB

        let content = String::from_utf8_lossy(&buffer);

        if content.contains("citygml/3.0") {
            Ok("3.0".to_string())
        } else if content.contains("citygml/2.0") {
            Ok("2.0".to_string())
        } else if content.contains("citygml/1.0") {
            Ok("1.0".to_string())
        } else if content.contains("www.opengis.net/gml") {
            Ok("2.0".to_string()) // Default to 2.0 for GML
        } else {
            Err(CityJsonStacError::MetadataError("Could not detect CityGML version".to_string()))
        }
    }

    fn stream_parse_metadata(path: &Path, metadata: &mut CityGMLMetadata) -> Result<()> {
        use std::io::BufReader;

        let file = std::fs::File::open(path)?;
        let reader = BufReader::new(file);
        let mut parser = quick_xml::Reader::from_reader(reader);

        let mut current_element = Vec::new();
        let mut in_city_object = false;
        let mut current_lod: Option<String> = None;

        loop {
            match parser.read_event_into(&mut current_element) {
                Ok(Event::Start(ref e)) => {
                    match e.name().as_ref() {
                        // CityGML namespace handling - look for common elements
                        b"core:CityObject" | b"cityObjectMember" | b"gen:CityObject" => {
                            in_city_object = true;
                            metadata.city_object_count += 1;
                        }
                        b"bldg:Building" | b"bldg:BuildingPart" | b"tran:Road" | b"tran:Rail" |
                        b"tran:Square" | b"tran:TINRelief" | b"wtr:WaterBody" => {
                            // Extract type name (namespace:local)
                            let type_name = std::str::from_utf8_lossy(e.name())
                                .split(':')
                                .last()
                                .unwrap_or(e.name())
                                .to_string();
                            metadata.city_object_types.insert(type_name);
                        }
                        b"bldg:lodXMultiSurface" | b"bldg:lodXSolid" | b"bldg:lodXImplicit" |
                        b"tran:lodXMultiSurface" => {
                            // Extract LOD number from element name
                            if let Some(lod_str) = std::str::from_utf8_lossy(e.name())
                                .split(':')
                                .last()
                            {
                                if let Some(ch) = lod_str.chars().next() {
                                    if ch.is_ascii_digit() {
                                        metadata.lods.insert(lod_str.to_string());
                                        current_lod = Some(lod_str.to_string());
                                    }
                                }
                            }
                        }
                        b"gml:boundedBy" => {
                            // Will extract bbox content when we get to it
                        }
                        _ => {}
                    }
                }
                Ok(Event::Text(ref text)) => {
                    // Check for semantic surfaces, textures, materials
                    let text_str = text.to_lowercase();
                    if text_str.contains("semanticsurface") || text_str.contains("semanticsurface") {
                        metadata.has_semantic_surfaces = true;
                    }
                    if text_str.contains("texture") || text_str.contains("textur") {
                        metadata.has_textures = true;
                    }
                    if text_str.contains("material") {
                        metadata.has_materials = true;
                    }
                }
                Ok(Event::End(_)) => {
                    in_city_object = false;
                    current_lod = None;
                }
                Ok(Event::Eof) => break,
                Err(e) => return Err(CityJsonStacError::Other(format!("XML error: {}", e))),
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
        if line.contains("citygml") || line.contains("www.opengis.net/gml") {
            return Ok(true);
        }
    }

    Ok(false)
}
```

### Files to Modify
- `Cargo.toml` - Add XML dependencies
- `src/reader/mod.rs` - Add .gml/.xml extension handling

### Files to Create
- `src/reader/citygml.rs` - CityGML reader implementation
- `tests/fixtures/sample_citygml_2.gml` - Test fixture
- `tests/fixtures/sample_citygml_3.gml` - Test fixture

### Testing
- Sample CityGML 2.0 and 3.0 test files
- Verify LOD extraction from different CityGML modules
- Test streaming parser with large files (>100MB)
- Verify XML namespace handling
- Test attribute extraction

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

### Phase 3: CityGML Reader

#### Milestone 3.0: CityGML Basic Reader
- [ ] Add quick-xml dependency
- [ ] Create `CityGMLReader` with streaming parser
- [ ] Implement version detection (2.0 vs 3.0)
- [ ] Implement `city_object_types()` extraction
- [ ] Implement `bbox()` extraction
- [ ] Implement `crs()` extraction
- [ ] Update factory for .gml/.xml files
- [ ] Tests with sample CityGML files

#### Milestone 3.5: CityGML Advanced Features
- [ ] LOD extraction for CityGML
- [ ] Attribute extraction from CityGML
- [ ] Handle CityGML 2.0 vs 3.0 differences
- [ ] Extension (ADE) detection
- [ ] Semantic surface detection
- [ ] Texture/material detection
- [ ] Large file streaming tests

---

## Dependencies Summary

### New Dependencies Required
| Crate | Version | Features | Phase |
|--------|----------|-----------|-------|
| serde_yaml | 0.9+ | - | 1 |
| tokio | 1.40+ | fs, rt-multi-thread | 2 |
| reqwest | 0.12+ | rustls-tls | 2 |
| quick-xml | 0.37+ | serialize | 3 |
| object_store | 0.11+ | http, aws, azure | 2.5 (optional) |
| bytes | 1.8+ | - | 2 |

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

1. **STAC Extension Prefix:** Already changed in commit #13 ✅
   - `cj:` → `city3d:` complete

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
3. **CHANGELOG.md** - Document breaking changes
4. **examples/** - Add config file examples

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
