//! STAC Item builder

use crate::error::{CityJsonStacError, Result};
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

    /// Add CityJSON extension properties from metadata reader
    pub fn cityjson_metadata(mut self, reader: &dyn CityModelMetadataReader) -> Result<Self> {
        // Add cj:encoding
        self.properties.insert(
            "cj:encoding".to_string(),
            Value::String(reader.encoding().to_string()),
        );

        // Add cj:version
        if let Ok(version) = reader.version() {
            self.properties
                .insert("cj:version".to_string(), Value::String(version));
        }

        // Add cj:city_objects
        if let Ok(count) = reader.city_object_count() {
            self.properties.insert(
                "cj:city_objects".to_string(),
                Value::Number(serde_json::Number::from(count)),
            );
        }

        // Add cj:lods
        if let Ok(lods) = reader.lods() {
            if !lods.is_empty() {
                self.properties
                    .insert("cj:lods".to_string(), serde_json::to_value(lods)?);
            }
        }

        // Add cj:co_types
        if let Ok(types) = reader.city_object_types() {
            if !types.is_empty() {
                self.properties
                    .insert("cj:co_types".to_string(), serde_json::to_value(types)?);
            }
        }

        // Add cj:attributes
        if let Ok(attrs) = reader.attributes() {
            if !attrs.is_empty() {
                self.properties
                    .insert("cj:attributes".to_string(), serde_json::to_value(attrs)?);
            }
        }

        // Add cj:transform
        if let Ok(Some(transform)) = reader.transform() {
            self.properties
                .insert("cj:transform".to_string(), serde_json::to_value(transform)?);
        }

        // Add cj:metadata
        if let Ok(Some(metadata)) = reader.metadata() {
            self.properties.insert("cj:metadata".to_string(), metadata);
        }

        // Add cj:extensions (CityJSON Application Domain Extensions)
        if let Ok(extensions) = reader.extensions() {
            if !extensions.is_empty() {
                self.properties.insert(
                    "cj:extensions".to_string(),
                    serde_json::to_value(extensions)?,
                );
            }
        }

        // Add proj:epsg from CRS
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
        // Validate that we have required CityJSON extension properties
        if !self.properties.contains_key("cj:encoding") {
            return Err(CityJsonStacError::StacError(
                "Missing required cj:encoding property".to_string(),
            ));
        }

        Ok(StacItem {
            stac_version: "1.0.0".to_string(),
            stac_extensions: vec![
                "https://raw.githubusercontent.com/cityjson/cityjson-stac/main/stac-extension/schema.json".to_string(),
                "https://stac-extensions.github.io/projection/v1.1.0/schema.json".to_string(),
            ],
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

        // Set bbox
        if let Ok(bbox) = reader.bbox() {
            builder = builder.bbox(bbox.clone()).geometry_from_bbox();
        }

        // Add CityJSON metadata
        builder = builder.cityjson_metadata(reader)?;

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
                    format!("{}/", base)
                };
                format!("{}{}", normalized_base, file_name)
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

        let id = format!("{}{}", stem, suffix);

        let mut builder = Self::new(id);

        // Set bbox
        if let Ok(bbox) = reader.bbox() {
            builder = builder.bbox(bbox.clone()).geometry_from_bbox();
        }

        // Add CityJSON metadata
        builder = builder.cityjson_metadata(reader)?;

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
                    format!("{}/", base)
                };
                format!("{}{}", normalized_base, file_name)
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
                        "boundaries": []
                    }],
                    "attributes": {
                        "yearOfConstruction": 2020
                    }
                }
            },
            "vertices": []
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
            .property(
                "cj:encoding".to_string(),
                Value::String("CityJSON".to_string()),
            )
            .build()
            .unwrap();

        assert_eq!(item.id, "test-item");
        assert_eq!(item.stac_version, "1.0.0");
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

        assert_eq!(item.properties.get("cj:encoding").unwrap(), "CityJSON");
        assert_eq!(item.properties.get("cj:version").unwrap(), "2.0");
        assert_eq!(item.properties.get("cj:city_objects").unwrap(), 1);
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
            .property(
                "cj:encoding".to_string(),
                Value::String("CityJSON".to_string()),
            )
            .build()
            .unwrap();

        assert!(item.geometry.is_some());
        let geom = item.geometry.unwrap();
        assert_eq!(geom["type"], "Polygon");
    }
}
