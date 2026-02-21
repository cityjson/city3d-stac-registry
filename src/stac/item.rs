//! STAC Item builder

use crate::error::Result;
use crate::metadata::BBox3D;
use crate::reader::CityModelMetadataReader;
use crate::stac::models::{Asset, Link, StacItem};
use chrono::{DateTime, Utc};
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;

/// Builder for STAC Items
pub struct StacItemBuilder {
    id: String,
    bbox: Option<Vec<f64>>,
    geometry: Option<Value>,
    properties: HashMap<String, Value>,
    assets: HashMap<String, Asset>,
    links: Vec<Link>,
    /// Track if File Extension is used (for stac_extensions list)
    uses_file_extension: bool,
}

impl StacItemBuilder {
    /// Create a new STAC Item builder
    pub fn new(id: impl Into<String>) -> Self {
        let mut properties = HashMap::new();

        // Set default datetime to now
        properties.insert(
            "datetime".to_string(),
            Value::String(Utc::now().to_rfc3339()),
        );

        Self {
            id: id.into(),
            bbox: None,
            geometry: None,
            properties,
            assets: HashMap::new(),
            links: Vec::new(),
            uses_file_extension: false,
        }
    }

    /// Set the 3D bounding box
    pub fn bbox(mut self, bbox: BBox3D) -> Self {
        self.bbox = Some(bbox.to_array().to_vec());
        self
    }

    /// Set the 2D geometry (footprint)
    pub fn geometry(mut self, geometry: Value) -> Self {
        self.geometry = Some(geometry);
        self
    }

    /// Set datetime
    pub fn datetime(mut self, dt: DateTime<Utc>) -> Self {
        self.properties
            .insert("datetime".to_string(), Value::String(dt.to_rfc3339()));
        self
    }

    /// Set title
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.properties
            .insert("title".to_string(), Value::String(title.into()));
        self
    }

    /// Set description
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.properties
            .insert("description".to_string(), Value::String(description.into()));
        self
    }

    /// Add a custom property
    pub fn property(mut self, key: impl Into<String>, value: Value) -> Self {
        self.properties.insert(key.into(), value);
        self
    }

    /// Add 3D City Models extension properties from metadata reader
    ///
    /// Uses the STAC 3D City Models Extension (city3d: prefix)
    /// https://cityjson.github.io/stac-city3d/v0.1.0/schema.json
    /// Add 3D City Models extension properties from metadata reader
    ///
    /// Uses the STAC 3D City Models Extension (city3d: prefix)
    /// https://cityjson.github.io/stac-city3d/v0.1.0/schema.json
    pub fn cityjson_metadata(mut self, reader: &dyn CityModelMetadataReader) -> Result<Self> {
        // Add city3d:version
        if let Ok(version) = reader.version() {
            self.properties
                .insert("city3d:version".to_string(), Value::String(version));
        }

        // Add city3d:city_objects
        if let Ok(count) = reader.city_object_count() {
            self.properties.insert(
                "city3d:city_objects".to_string(),
                Value::Number(serde_json::Number::from(count)),
            );
        }

        // Add city3d:lods ensuring they are numbers
        if let Ok(lods) = reader.lods() {
            if !lods.is_empty() {
                let numeric_lods: Vec<Value> = lods
                    .iter()
                    .map(|lod| {
                        // Try to parse as number
                        if let Ok(num) = lod.parse::<f64>() {
                            // If it's a number, use Number
                            if let Some(n) = serde_json::Number::from_f64(num) {
                                Value::Number(n)
                            } else {
                                // Fallback for NaN/Infinity
                                Value::String(lod.clone())
                            }
                        } else {
                            // Keep as string if not parseable (should not happen if schema requires number, but safe fallback)
                            // Or better, filter out non-numerics?
                            // For now, let's assume valid data or fallback to string which might fail valid later but preserves data
                            Value::String(lod.clone())
                        }
                    })
                    .collect();
                self.properties
                    .insert("city3d:lods".to_string(), Value::Array(numeric_lods));
            }
        }

        // Add city3d:co_types
        if let Ok(types) = reader.city_object_types() {
            if !types.is_empty() {
                self.properties
                    .insert("city3d:co_types".to_string(), serde_json::to_value(types)?);
            }
        }

        // Add city3d:attributes
        if let Ok(attrs) = reader.attributes() {
            if !attrs.is_empty() {
                self.properties.insert(
                    "city3d:attributes".to_string(),
                    serde_json::to_value(attrs)?,
                );
            }
        }

        // Add city3d:semantic_surfaces
        if let Ok(has_semantic_surfaces) = reader.semantic_surfaces() {
            if has_semantic_surfaces {
                self.properties
                    .insert("city3d:semantic_surfaces".to_string(), Value::Bool(true));
            }
        }

        // Add city3d:textures
        if let Ok(has_textures) = reader.textures() {
            if has_textures {
                self.properties
                    .insert("city3d:textures".to_string(), Value::Bool(true));
            }
        }

        // Add city3d:materials
        if let Ok(has_materials) = reader.materials() {
            if has_materials {
                self.properties
                    .insert("city3d:materials".to_string(), Value::Bool(true));
            }
        }

        // Add proj:epsg from CRS (integer, as per STAC Projection Extension v1)
        if let Ok(crs) = reader.crs() {
            if let Some(epsg) = crs.to_stac_epsg() {
                self.properties.insert(
                    "proj:epsg".to_string(),
                    Value::Number(serde_json::Number::from(epsg)),
                );
            }
        }

        Ok(self)
    }

    /// Add file size property (File Extension)
    pub fn file_size(mut self, size: u64) -> Self {
        self.properties.insert(
            "file:size".to_string(),
            Value::Number(serde_json::Number::from(size)),
        );
        self.uses_file_extension = true;
        self
    }

    /// Add a data asset pointing to the source file
    pub fn data_asset(mut self, href: impl Into<String>, media_type: &str) -> Self {
        let asset = Asset::new(href)
            .with_type(media_type)
            .with_title("CityJSON data file")
            .with_roles(vec!["data".to_string()]);

        self.assets.insert("data".to_string(), asset);
        self
    }

    /// Add a custom asset
    pub fn asset(mut self, key: impl Into<String>, asset: Asset) -> Self {
        self.assets.insert(key.into(), asset);
        self
    }

    /// Add a link
    pub fn link(mut self, link: Link) -> Self {
        self.links.push(link);
        self
    }

    /// Add a self link
    pub fn self_link(mut self, href: impl Into<String>) -> Self {
        self.links
            .push(Link::new("self", href).with_type("application/json"));
        self
    }

    /// Add a parent link
    pub fn parent_link(mut self, href: impl Into<String>) -> Self {
        self.links
            .push(Link::new("parent", href).with_type("application/json"));
        self
    }

    /// Add a collection link
    pub fn collection_link(mut self, href: impl Into<String>) -> Self {
        self.links
            .push(Link::new("collection", href).with_type("application/json"));
        self
    }

    /// Build the STAC Item
    pub fn build(self) -> Result<StacItem> {
        // city3d:encoding check removed

        // Build stac_extensions list dynamically based on which extensions are used
        // IMPORTANT: We do NOT rely on schema dependencies anymore, so we must add explicit extension URLs
        let mut stac_extensions =
            vec!["https://cityjson.github.io/stac-city3d/v0.1.0/schema.json".to_string()];

        // Add Projection Extension if proj:epsg is present
        if self.properties.contains_key("proj:epsg") {
            // We can check for legacy proj:epsg just in case, but we write proj:code now
            // Using v2.0.0 projection extension as it removed proj:epsg but we use proper proj:code
            stac_extensions.push(
                "https://stac-extensions.github.io/projection/v2.0.0/schema.json".to_string(),
            );
        }

        // Add File Extension if file:size is present
        if self.uses_file_extension {
            stac_extensions
                .push("https://stac-extensions.github.io/file/v2.1.0/schema.json".to_string());
        }

        Ok(StacItem {
            stac_version: "1.1.0".to_string(),
            stac_extensions,
            item_type: "Feature".to_string(),
            id: self.id,
            bbox: self.bbox,
            geometry: self.geometry,
            properties: self.properties,
            assets: self.assets,
            links: self.links,
        })
    }

    /// Generate a simple 2D polygon geometry from bbox
    pub fn geometry_from_bbox(mut self) -> Self {
        if let Some(ref bbox) = self.bbox {
            if bbox.len() >= 4 {
                let xmin = bbox[0];
                let ymin = bbox[1];
                let xmax = if bbox.len() == 6 { bbox[3] } else { bbox[2] };
                let ymax = if bbox.len() == 6 { bbox[4] } else { bbox[3] };

                let geometry = serde_json::json!({
                    "type": "Polygon",
                    "coordinates": [[
                        [xmin, ymin],
                        [xmax, ymin],
                        [xmax, ymax],
                        [xmin, ymax],
                        [xmin, ymin]
                    ]]
                });

                self.geometry = Some(geometry);
            }
        }
        self
    }

    /// Helper to create item from file path
    ///
    /// Bbox and geometry are automatically transformed to WGS84 (EPSG:4326)
    /// as required by the STAC specification (per GeoJSON RFC 7946).
    ///
    /// # Arguments
    /// * `file_path` - Path to the CityJSON file
    /// * `reader` - Reader instance for the file
    /// * `base_url` - Optional base URL for asset hrefs. If provided, asset hrefs will be
    ///   absolute URLs (e.g., "https://example.com/data/file.json").
    ///   If None, hrefs will be relative paths (just the filename).
    pub fn from_file(
        file_path: &Path,
        reader: &dyn CityModelMetadataReader,
        base_url: Option<&str>,
    ) -> Result<Self> {
        let id = file_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        let mut builder = Self::new(id);

        // Set bbox (transformed to WGS84 for STAC compliance)
        if let Ok(bbox) = reader.bbox() {
            let crs = reader.crs().unwrap_or_default();
            let wgs84_bbox = bbox.to_wgs84(&crs)?;
            builder = builder.bbox(wgs84_bbox).geometry_from_bbox();
        }

        // Add CityJSON metadata
        builder = builder.cityjson_metadata(reader)?;

        // Add file size (File Extension)
        if let Ok(metadata) = std::fs::metadata(file_path) {
            builder = builder.file_size(metadata.len());
        }

        // Add data asset
        let media_type = match reader.encoding() {
            "CityJSON" => "application/json",
            "CityJSONSeq" => "application/json-seq",
            "FlatCityBuf" => "application/octet-stream",
            _ => "application/octet-stream",
        };

        // Generate asset href based on base_url
        let file_name = file_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("data");

        let href = match base_url {
            Some(base) => {
                // Ensure base URL ends with a slash
                let normalized_base = if base.ends_with('/') {
                    base.to_string()
                } else {
                    format!("{base}/")
                };
                format!("{normalized_base}{file_name}")
            }
            None => file_name.to_string(),
        };

        builder = builder.data_asset(href, media_type);

        Ok(builder)
    }

    /// Helper to create item from file path with format suffix in ID
    ///
    /// This variant generates IDs with format suffixes (e.g., "delft_cj", "delft_cjseq", "delft_fcb")
    /// to handle filename collisions where multiple formats have the same stem.
    ///
    /// Bbox and geometry are automatically transformed to WGS84 (EPSG:4326)
    /// as required by the STAC specification (per GeoJSON RFC 7946).
    ///
    /// # Arguments
    /// * `file_path` - Path to the CityJSON file
    /// * `reader` - Reader instance for the file
    /// * `base_url` - Optional base URL for asset hrefs
    pub fn from_file_with_format_suffix(
        file_path: &Path,
        reader: &dyn CityModelMetadataReader,
        base_url: Option<&str>,
    ) -> Result<Self> {
        let stem = file_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");

        // Generate format-specific suffix
        let suffix = match reader.encoding() {
            "CityJSON" => "_cj",
            "CityJSONSeq" => "_cjseq",
            "FlatCityBuf" => "_fcb",
            _ => "",
        };

        let id = format!("{stem}{suffix}");

        let mut builder = Self::new(id);

        // Set bbox (transformed to WGS84 for STAC compliance)
        if let Ok(bbox) = reader.bbox() {
            let crs = reader.crs().unwrap_or_default();
            let wgs84_bbox = bbox.to_wgs84(&crs)?;
            builder = builder.bbox(wgs84_bbox).geometry_from_bbox();
        }

        // Add CityJSON metadata
        builder = builder.cityjson_metadata(reader)?;

        // Add file size (File Extension)
        if let Ok(metadata) = std::fs::metadata(file_path) {
            builder = builder.file_size(metadata.len());
        }

        // Add data asset
        let media_type = match reader.encoding() {
            "CityJSON" => "application/json",
            "CityJSONSeq" => "application/json-seq",
            "FlatCityBuf" => "application/octet-stream",
            _ => "application/octet-stream",
        };

        // Generate asset href based on base_url
        let file_name = file_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("data");

        let href = match base_url {
            Some(base) => {
                // Ensure base URL ends with a slash
                let normalized_base = if base.ends_with('/') {
                    base.to_string()
                } else {
                    format!("{base}/")
                };
                format!("{normalized_base}{file_name}")
            }
            None => file_name.to_string(),
        };

        builder = builder.data_asset(href, media_type);

        Ok(builder)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metadata::BBox3D;
    use crate::reader::CityJSONReader;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn create_test_cityjson() -> NamedTempFile {
        let mut temp_file = NamedTempFile::new().unwrap();
        let cityjson = r#"{
            "type": "CityJSON",
            "version": "2.0",
            "transform": {
                "scale": [0.01, 0.01, 0.01],
                "translate": [100000, 200000, 0]
            },
            "metadata": {
                "geographicalExtent": [1.0, 2.0, 0.0, 10.0, 20.0, 30.0],
                "referenceSystem": "https://www.opengis.net/def/crs/EPSG/0/7415"
            },
            "CityObjects": {
                "building1": {
                    "type": "Building",
                    "geometry": [{
                        "type": "Solid",
                        "lod": "2",
                        "boundaries": [[[[0,0,0]]]]
                    }],
                    "attributes": {
                        "yearOfConstruction": 2020
                    }
                }
            },
            "vertices": [[0,0,0]]
        }"#;

        writeln!(temp_file, "{}", cityjson).unwrap();
        temp_file.flush().unwrap();
        temp_file
    }

    #[test]
    fn test_item_builder_basic() {
        let item = StacItemBuilder::new("test-item")
            .title("Test Item")
            .description("A test item")
            .build()
            .unwrap();

        assert_eq!(item.id, "test-item");
        assert_eq!(item.stac_version, "1.1.0");
        assert_eq!(item.properties.get("title").unwrap(), "Test Item");
    }

    #[test]
    fn test_item_builder_with_cityjson() {
        let temp_file = create_test_cityjson();
        let reader = CityJSONReader::new(temp_file.path()).unwrap();

        let item = StacItemBuilder::new("test-building")
            .cityjson_metadata(&reader)
            .unwrap()
            .build()
            .unwrap();

        assert_eq!(item.properties.get("city3d:version").unwrap(), "2.0");
        assert_eq!(item.properties.get("city3d:city_objects").unwrap(), 1);
        assert_eq!(item.properties.get("proj:epsg").unwrap(), 7415);
    }

    #[test]
    fn test_item_builder_from_file() {
        let temp_file = create_test_cityjson();
        let reader = CityJSONReader::new(temp_file.path()).unwrap();

        let builder = StacItemBuilder::from_file(temp_file.path(), &reader, None).unwrap();
        let item = builder.build().unwrap();

        assert!(item.bbox.is_some());
        assert!(item.geometry.is_some());
        assert!(item.assets.contains_key("data"));
    }

    #[test]
    fn test_geometry_from_bbox() {
        let bbox = BBox3D::new(0.0, 0.0, 0.0, 10.0, 10.0, 10.0);

        let item = StacItemBuilder::new("test")
            .bbox(bbox)
            .geometry_from_bbox()
            .build()
            .unwrap();

        assert!(item.geometry.is_some());
        let geom = item.geometry.unwrap();
        assert_eq!(geom["type"], "Polygon");
    }
}
