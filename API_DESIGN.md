# CityJSON-STAC API Design

## Trait Definitions

### Core Metadata Reader Trait

```rust
use std::path::Path;
use crate::metadata::{BBox3D, CRS, AttributeDefinition};
use crate::error::Result;

/// Trait for extracting metadata from CityJSON-format files
///
/// Implemented by format-specific readers (CityJSON, CityJSONSeq, FlatCityBuf, etc.)
pub trait CityModelMetadataReader: Send + Sync {
    /// Extract 3D bounding box [xmin, ymin, zmin, xmax, ymax, zmax]
    ///
    /// Returns the spatial extent of all geometry in the file.
    /// Values should be in the native CRS of the dataset.
    fn bbox(&self) -> Result<BBox3D>;

    /// Get coordinate reference system information
    ///
    /// Returns EPSG code and WKT2 representation if available
    fn crs(&self) -> Result<CRS>;

    /// Get list of available Levels of Detail
    ///
    /// Returns strings like ["0", "1", "2", "2.2"]
    fn lods(&self) -> Result<Vec<String>>;

    /// Get list of CityObject types present
    ///
    /// Returns types like ["Building", "BuildingPart", "Road"]
    fn city_object_types(&self) -> Result<Vec<String>>;

    /// Count total number of city objects
    fn city_object_count(&self) -> Result<usize>;

    /// Extract attribute schema definitions
    ///
    /// Returns schema describing semantic attributes attached to objects
    fn attributes(&self) -> Result<Vec<AttributeDefinition>>;

    /// Get encoding format name
    ///
    /// Returns one of: "CityJSON", "CityJSONSeq", "FlatCityBuf", "CityParquet"
    fn encoding(&self) -> &'static str;

    /// Get CityJSON version
    ///
    /// Returns version string like "2.0" or "1.1"
    fn version(&self) -> Result<String>;

    /// Get file path being read
    fn file_path(&self) -> &Path;

    /// Get coordinate transform parameters if present
    ///
    /// Returns scale and translate arrays for vertex compression
    fn transform(&self) -> Result<Option<Transform>>;

    /// Extract additional metadata from file
    ///
    /// Returns free-form metadata object from CityJSON
    fn metadata(&self) -> Result<Option<serde_json::Value>>;
}
```

### Reader Factory

```rust
use std::path::Path;
use crate::error::{Result, CityJsonStacError};

/// Factory function to create appropriate reader for a file
///
/// # Arguments
/// * `file_path` - Path to the file to read
///
/// # Returns
/// Boxed trait object implementing CityModelMetadataReader
///
/// # Errors
/// Returns error if file format is unsupported or file cannot be opened
pub fn get_reader(file_path: &Path) -> Result<Box<dyn CityModelMetadataReader>> {
    let extension = file_path
        .extension()
        .and_then(|e| e.to_str())
        .ok_or_else(|| CityJsonStacError::UnsupportedFormat(
            "No file extension found".to_string()
        ))?;

    match extension.to_lowercase().as_str() {
        "json" => {
            // Need to peek inside to distinguish CityJSON from regular JSON
            if is_cityjson(file_path)? {
                Ok(Box::new(CityJSONReader::new(file_path)?))
            } else {
                Err(CityJsonStacError::UnsupportedFormat(
                    "Not a CityJSON file".to_string()
                ))
            }
        }
        "jsonl" | "cjseq" => Ok(Box::new(CityJSONSeqReader::new(file_path)?)),
        "fcb" => Ok(Box::new(FlatCityBufReader::new(file_path)?)),
        "parquet" => {
            // Future support
            Err(CityJsonStacError::UnsupportedFormat(
                "CityParquet not yet supported".to_string()
            ))
        }
        _ => Err(CityJsonStacError::UnsupportedFormat(
            format!("Unknown extension: {}", extension)
        )),
    }
}

/// Helper to check if JSON file is CityJSON format
fn is_cityjson(file_path: &Path) -> Result<bool> {
    // Read first few KB and check for "type": "CityJSON" field
    // Implementation details omitted
    todo!()
}
```

## Metadata Structures

### BBox3D

```rust
use serde::{Serialize, Deserialize};

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
    pub fn new(xmin: f64, ymin: f64, zmin: f64, xmax: f64, ymax: f64, zmax: f64) -> Self {
        Self { xmin, ymin, zmin, xmax, ymax, zmax }
    }

    /// Convert to STAC bbox array format
    pub fn to_array(&self) -> [f64; 6] {
        [self.xmin, self.ymin, self.zmin, self.xmax, self.ymax, self.zmax]
    }

    /// Merge two bounding boxes (union)
    pub fn merge(&self, other: &BBox3D) -> BBox3D {
        BBox3D {
            xmin: self.xmin.min(other.xmin),
            ymin: self.ymin.min(other.ymin),
            zmin: self.zmin.min(other.zmin),
            xmax: self.xmax.max(other.xmax),
            ymax: self.ymax.max(other.ymax),
            zmax: self.zmax.max(other.zmax),
        }
    }

    /// Get 2D footprint (for STAC geometry)
    pub fn footprint_2d(&self) -> [f64; 4] {
        [self.xmin, self.ymin, self.xmax, self.ymax]
    }
}
```

### CRS

```rust
/// Coordinate Reference System information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CRS {
    /// EPSG code (e.g., 7415)
    pub epsg: Option<u32>,

    /// WKT2 representation
    pub wkt2: Option<String>,

    /// PROJ.4 string
    pub proj4: Option<String>,

    /// CityJSON authority/identifier format
    pub authority: Option<String>,
    pub identifier: Option<String>,
}

impl CRS {
    pub fn from_epsg(code: u32) -> Self {
        Self {
            epsg: Some(code),
            wkt2: None,
            proj4: None,
            authority: Some("EPSG".to_string()),
            identifier: Some(code.to_string()),
        }
    }

    /// Get best available CRS representation for STAC
    pub fn to_stac_epsg(&self) -> Option<u32> {
        self.epsg
    }
}
```

### AttributeDefinition

```rust
/// Schema definition for a semantic attribute
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributeDefinition {
    /// Attribute name
    pub name: String,

    /// Data type: String, Number, Boolean, Date, Array, Object
    #[serde(rename = "type")]
    pub attr_type: AttributeType,

    /// Human-readable description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Whether attribute is always present
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AttributeType {
    String,
    Number,
    Boolean,
    Date,
    Array,
    Object,
}
```

### Transform

```rust
/// Coordinate transform parameters for vertex compression
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transform {
    pub scale: [f64; 3],
    pub translate: [f64; 3],
}

impl Transform {
    pub fn new(scale: [f64; 3], translate: [f64; 3]) -> Self {
        Self { scale, translate }
    }

    /// Apply transform to compressed coordinate
    pub fn apply(&self, compressed: &[i32; 3]) -> [f64; 3] {
        [
            compressed[0] as f64 * self.scale[0] + self.translate[0],
            compressed[1] as f64 * self.scale[1] + self.translate[1],
            compressed[2] as f64 * self.scale[2] + self.translate[2],
        ]
    }
}
```

## STAC Builder API

### STAC Item Builder

```rust
use chrono::{DateTime, Utc};
use geojson::Geometry;

/// Builder for STAC Items
pub struct StacItemBuilder {
    id: String,
    geometry: Option<Geometry>,
    bbox: Option<Vec<f64>>,
    properties: serde_json::Map<String, serde_json::Value>,
    assets: serde_json::Map<String, serde_json::Value>,
    links: Vec<Link>,
}

impl StacItemBuilder {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            geometry: None,
            bbox: None,
            properties: serde_json::Map::new(),
            assets: serde_json::Map::new(),
            links: Vec::new(),
        }
    }

    /// Set bounding box
    pub fn bbox(mut self, bbox: BBox3D) -> Self {
        self.bbox = Some(bbox.to_array().to_vec());
        self
    }

    /// Set 2D geometry (footprint)
    pub fn geometry(mut self, geom: Geometry) -> Self {
        self.geometry = Some(geom);
        self
    }

    /// Set datetime
    pub fn datetime(mut self, dt: DateTime<Utc>) -> Self {
        self.properties.insert(
            "datetime".to_string(),
            serde_json::Value::String(dt.to_rfc3339())
        );
        self
    }

    /// Set title
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.properties.insert("title".to_string(), title.into().into());
        self
    }

    /// Set description
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.properties.insert("description".to_string(), desc.into().into());
        self
    }

    /// Add CityJSON extension properties from metadata reader
    pub fn cityjson_metadata(mut self, reader: &dyn CityModelMetadataReader) -> Result<Self> {
        // Add cj:encoding
        self.properties.insert(
            "cj:encoding".to_string(),
            reader.encoding().into()
        );

        // Add cj:version
        if let Ok(version) = reader.version() {
            self.properties.insert("cj:version".to_string(), version.into());
        }

        // Add cj:city_objects
        if let Ok(count) = reader.city_object_count() {
            self.properties.insert("cj:city_objects".to_string(), (count as i64).into());
        }

        // Add cj:lods
        if let Ok(lods) = reader.lods() {
            self.properties.insert("cj:lods".to_string(), lods.into());
        }

        // Add cj:co_types
        if let Ok(types) = reader.city_object_types() {
            self.properties.insert("cj:co_types".to_string(), types.into());
        }

        // Add cj:attributes
        if let Ok(attrs) = reader.attributes() {
            self.properties.insert(
                "cj:attributes".to_string(),
                serde_json::to_value(attrs)?
            );
        }

        // Add cj:transform
        if let Ok(Some(transform)) = reader.transform() {
            self.properties.insert(
                "cj:transform".to_string(),
                serde_json::to_value(transform)?
            );
        }

        // Add proj:epsg
        if let Ok(crs) = reader.crs() {
            if let Some(epsg) = crs.to_stac_epsg() {
                self.properties.insert("proj:epsg".to_string(), (epsg as i64).into());
            }
        }

        Ok(self)
    }

    /// Add data asset
    pub fn data_asset(mut self, href: String, media_type: &str) -> Self {
        let mut asset = serde_json::Map::new();
        asset.insert("href".to_string(), href.into());
        asset.insert("type".to_string(), media_type.into());
        asset.insert("roles".to_string(), vec!["data"].into());

        self.assets.insert("data".to_string(), asset.into());
        self
    }

    /// Add link
    pub fn link(mut self, rel: String, href: String) -> Self {
        self.links.push(Link { rel, href, link_type: None, title: None });
        self
    }

    /// Build the STAC Item
    pub fn build(self) -> Result<StacItem> {
        Ok(StacItem {
            stac_version: "1.0.0".to_string(),
            stac_extensions: vec![
                "https://raw.githubusercontent.com/cityjson/cityjson-stac/main/stac-extension/schema.json".to_string(),
                "https://stac-extensions.github.io/projection/v1.1.0/schema.json".to_string(),
            ],
            item_type: "Feature".to_string(),
            id: self.id,
            geometry: self.geometry,
            bbox: self.bbox,
            properties: self.properties,
            assets: self.assets,
            links: self.links,
        })
    }
}
```

### STAC Collection Builder

```rust
/// Builder for STAC Collections
pub struct StacCollectionBuilder {
    id: String,
    title: Option<String>,
    description: Option<String>,
    license: String,
    extent: Extent,
    summaries: serde_json::Map<String, serde_json::Value>,
    links: Vec<Link>,
}

impl StacCollectionBuilder {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: None,
            description: None,
            license: "proprietary".to_string(),
            extent: Extent::default(),
            summaries: serde_json::Map::new(),
            links: Vec::new(),
        }
    }

    /// Set title
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set description
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Set license
    pub fn license(mut self, license: impl Into<String>) -> Self {
        self.license = license.into();
        self
    }

    /// Set spatial extent
    pub fn spatial_extent(mut self, bbox: BBox3D) -> Self {
        self.extent.spatial.bbox.push(bbox.to_array().to_vec());
        self
    }

    /// Set temporal extent
    pub fn temporal_extent(mut self, start: Option<DateTime<Utc>>, end: Option<DateTime<Utc>>) -> Self {
        let start_str = start.map(|dt| dt.to_rfc3339());
        let end_str = end.map(|dt| dt.to_rfc3339());
        self.extent.temporal.interval.push(vec![start_str, end_str]);
        self
    }

    /// Aggregate metadata from multiple readers
    pub fn aggregate_cityjson_metadata(mut self, readers: &[Box<dyn CityModelMetadataReader>]) -> Result<Self> {
        // Collect all encodings
        let encodings: Vec<String> = readers.iter()
            .map(|r| r.encoding().to_string())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        self.summaries.insert("cj:encoding".to_string(), encodings.into());

        // Aggregate LODs
        let all_lods: Vec<String> = readers.iter()
            .filter_map(|r| r.lods().ok())
            .flatten()
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        self.summaries.insert("cj:lods".to_string(), all_lods.into());

        // Aggregate city object types
        let all_types: Vec<String> = readers.iter()
            .filter_map(|r| r.city_object_types().ok())
            .flatten()
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        self.summaries.insert("cj:co_types".to_string(), all_types.into());

        // City object count statistics
        let counts: Vec<usize> = readers.iter()
            .filter_map(|r| r.city_object_count().ok())
            .collect();
        if !counts.is_empty() {
            let mut stats = serde_json::Map::new();
            stats.insert("min".to_string(), (*counts.iter().min().unwrap() as i64).into());
            stats.insert("max".to_string(), (*counts.iter().max().unwrap() as i64).into());
            stats.insert("total".to_string(), (counts.iter().sum::<usize>() as i64).into());
            self.summaries.insert("cj:city_objects".to_string(), stats.into());
        }

        Ok(self)
    }

    /// Add item link
    pub fn item_link(mut self, href: String) -> Self {
        self.links.push(Link {
            rel: "item".to_string(),
            href,
            link_type: Some("application/json".to_string()),
            title: None,
        });
        self
    }

    /// Build the STAC Collection
    pub fn build(self) -> Result<StacCollection> {
        Ok(StacCollection {
            stac_version: "1.0.0".to_string(),
            stac_extensions: vec![
                "https://raw.githubusercontent.com/cityjson/cityjson-stac/main/stac-extension/schema.json".to_string(),
            ],
            collection_type: "Collection".to_string(),
            id: self.id,
            title: self.title,
            description: self.description,
            license: self.license,
            extent: self.extent,
            summaries: self.summaries,
            links: self.links,
        })
    }
}
```

## Reader Implementations

### CityJSON Reader Skeleton

```rust
use std::path::{Path, PathBuf};
use std::fs::File;
use std::io::BufReader;
use serde_json::Value;

pub struct CityJSONReader {
    file_path: PathBuf,
    // Cached parsed data (lazy loaded)
    data: Option<Value>,
}

impl CityJSONReader {
    pub fn new(file_path: &Path) -> Result<Self> {
        if !file_path.exists() {
            return Err(CityJsonStacError::IoError(
                std::io::Error::new(std::io::ErrorKind::NotFound, "File not found")
            ));
        }

        Ok(Self {
            file_path: file_path.to_path_buf(),
            data: None,
        })
    }

    /// Lazy load and cache JSON data
    fn ensure_loaded(&mut self) -> Result<&Value> {
        if self.data.is_none() {
            let file = File::open(&self.file_path)?;
            let reader = BufReader::new(file);
            self.data = Some(serde_json::from_reader(reader)?);
        }
        Ok(self.data.as_ref().unwrap())
    }
}

impl CityModelMetadataReader for CityJSONReader {
    fn bbox(&self) -> Result<BBox3D> {
        // Parse "metadata" -> "geographicalExtent" field
        // or compute from vertices
        todo!()
    }

    fn crs(&self) -> Result<CRS> {
        // Parse "metadata" -> "referenceSystem" field
        todo!()
    }

    fn lods(&self) -> Result<Vec<String>> {
        // Scan all CityObjects geometry and collect unique LODs
        todo!()
    }

    fn city_object_types(&self) -> Result<Vec<String>> {
        // Scan all CityObjects and collect unique "type" values
        todo!()
    }

    fn city_object_count(&self) -> Result<usize> {
        // Count entries in "CityObjects" map
        todo!()
    }

    fn attributes(&self) -> Result<Vec<AttributeDefinition>> {
        // Scan CityObjects attributes and build schema
        todo!()
    }

    fn encoding(&self) -> &'static str {
        "CityJSON"
    }

    fn version(&self) -> Result<String> {
        // Parse "version" field
        todo!()
    }

    fn file_path(&self) -> &Path {
        &self.file_path
    }

    fn transform(&self) -> Result<Option<Transform>> {
        // Parse "transform" field if present
        todo!()
    }

    fn metadata(&self) -> Result<Option<Value>> {
        // Return "metadata" object
        todo!()
    }
}
```

### FlatCityBuf Reader Skeleton

```rust
pub struct FlatCityBufReader {
    file_path: PathBuf,
    // FlatBuffers-specific data structures
}

impl FlatCityBufReader {
    pub fn new(file_path: &Path) -> Result<Self> {
        // Use flatcitybuf crate to open file
        todo!()
    }
}

impl CityModelMetadataReader for FlatCityBufReader {
    // Implement using FlatBuffers API
    // Reference: https://github.com/cityjson/flatcitybuf
    fn encoding(&self) -> &'static str {
        "FlatCityBuf"
    }

    // ... other methods
}
```

## Error Handling

```rust
use thiserror::Error;

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

pub type Result<T> = std::result::Result<T, CityJsonStacError>;
```

## Usage Examples

### Generate Single Item

```rust
use cityjson_stac::reader::get_reader;
use cityjson_stac::stac::StacItemBuilder;

fn generate_item(file_path: &Path) -> Result<()> {
    // Get appropriate reader
    let reader = get_reader(file_path)?;

    // Extract metadata and build STAC Item
    let item = StacItemBuilder::new("my_building_model")
        .bbox(reader.bbox()?)
        .datetime(Utc::now())
        .title("Building Model")
        .cityjson_metadata(reader.as_ref())?
        .data_asset(
            file_path.to_string_lossy().to_string(),
            "application/json"
        )
        .build()?;

    // Serialize to JSON
    let json = serde_json::to_string_pretty(&item)?;
    std::fs::write("output_item.json", json)?;

    Ok(())
}
```

### Generate Collection from Directory

```rust
use walkdir::WalkDir;

fn generate_collection(dir_path: &Path, output_dir: &Path) -> Result<()> {
    let mut readers: Vec<Box<dyn CityModelMetadataReader>> = Vec::new();
    let mut merged_bbox: Option<BBox3D> = None;

    // Traverse directory
    for entry in WalkDir::new(dir_path).follow_links(true) {
        let entry = entry?;
        if entry.file_type().is_file() {
            if let Ok(reader) = get_reader(entry.path()) {
                // Merge bounding boxes
                if let Ok(bbox) = reader.bbox() {
                    merged_bbox = Some(match merged_bbox {
                        Some(existing) => existing.merge(&bbox),
                        None => bbox,
                    });
                }

                readers.push(reader);
            }
        }
    }

    // Build collection
    let collection = StacCollectionBuilder::new("my_collection")
        .title("City Buildings")
        .description("Collection of building models")
        .spatial_extent(merged_bbox.unwrap())
        .temporal_extent(Some(Utc::now()), None)
        .aggregate_cityjson_metadata(&readers)?
        .build()?;

    // Write collection JSON
    let json = serde_json::to_string_pretty(&collection)?;
    std::fs::write(output_dir.join("collection.json"), json)?;

    Ok(())
}
```
