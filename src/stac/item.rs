//! STAC Item builder

use crate::error::Result;
use crate::metadata::BBox3D;
use crate::metadata::CRS;
use crate::reader::CityModelMetadataReader;
use chrono::{DateTime, Utc};
use indexmap::IndexMap;
use serde_json::Value;
use std::path::Path;

/// Map encoding name to IANA/vendor media type
fn encoding_media_type(encoding: &str) -> &'static str {
    match encoding {
        "CityJSON" => "application/city+json",
        "CityJSONSeq" => "application/city+json-seq",
        "CityGML" => "application/gml+xml",
        "FlatCityBuf" => "application/vnd.flatcitybuf",
        _ => "application/octet-stream",
    }
}

/// Builder for STAC Items
pub struct StacItemBuilder {
    id: String,
    bbox: Option<Vec<f64>>,
    geometry: Option<Value>,
    properties: serde_json::Map<String, Value>,
    datetime: Option<DateTime<Utc>>,
    start_datetime: Option<DateTime<Utc>>,
    end_datetime: Option<DateTime<Utc>>,
    title: Option<String>,
    description: Option<String>,
    assets: IndexMap<String, stac::Asset>,
    links: Vec<stac::Link>,
    /// Track if File Extension is used (for stac_extensions list)
    uses_file_extension: bool,
}

impl StacItemBuilder {
    /// Create a new STAC Item builder
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            bbox: None,
            geometry: None,
            properties: serde_json::Map::new(),
            datetime: None,
            start_datetime: None,
            end_datetime: None,
            title: None,
            description: None,
            assets: IndexMap::new(),
            links: Vec::new(),
            uses_file_extension: false,
        }
    }

    /// Resolve CRS from reader, using the override as fallback when the reader's CRS is unknown.
    fn resolve_crs(reader: &dyn CityModelMetadataReader, crs_override: Option<&CRS>) -> CRS {
        let crs = reader.crs().unwrap_or_default();
        if crs.is_known() {
            crs
        } else if let Some(override_crs) = crs_override {
            override_crs.clone()
        } else {
            crs
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

    /// Set datetime as an RFC3339 string, or null if None
    pub fn datetime(mut self, dt: Option<String>) -> Self {
        self.datetime = dt.and_then(|s| s.parse::<DateTime<Utc>>().ok());
        self
    }

    /// Set start_datetime (used when datetime is null and a date range is specified)
    pub fn start_datetime(mut self, dt: impl Into<String>) -> Self {
        self.start_datetime = dt.into().parse::<DateTime<Utc>>().ok();
        self
    }

    /// Set end_datetime (used when datetime is null and a date range is specified)
    pub fn end_datetime(mut self, dt: impl Into<String>) -> Self {
        self.end_datetime = dt.into().parse::<DateTime<Utc>>().ok();
        self
    }

    /// Set title
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set description
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
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
    pub fn cityjson_metadata(mut self, reader: &dyn CityModelMetadataReader) -> Result<Self> {
        // Extract referenceDate from CityJSON metadata → set as datetime
        if let Ok(Some(metadata)) = reader.metadata() {
            if let Some(ref_date) = metadata.get("referenceDate").and_then(|v| v.as_str()) {
                // referenceDate is typically "YYYY-MM-DD"; convert to RFC3339
                let datetime_str = if ref_date.contains('T') {
                    ref_date.to_string()
                } else {
                    format!("{ref_date}T00:00:00Z")
                };
                self.datetime = datetime_str.parse::<DateTime<Utc>>().ok();
            }
        }

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
                        if let Ok(num) = lod.parse::<f64>() {
                            if let Some(n) = serde_json::Number::from_f64(num) {
                                Value::Number(n)
                            } else {
                                Value::String(lod.clone())
                            }
                        } else {
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

        // Add proj:code from CRS (string, as per STAC Projection Extension v2.0.0)
        if let Ok(crs) = reader.crs() {
            if let Some(proj_code) = crs.to_stac_proj_code() {
                self.properties
                    .insert("proj:code".to_string(), Value::String(proj_code));
            }
        }

        Ok(self)
    }

    /// Add a data asset pointing to the source file
    ///
    /// Optionally accepts a file size which is placed on the asset as `file:size`
    /// per the STAC File Extension spec (file extension fields belong on assets, not item properties).
    pub fn data_asset(
        mut self,
        href: impl Into<String>,
        media_type: &str,
        file_size: Option<u64>,
    ) -> Self {
        let mut asset = stac::Asset::new(href.into());
        asset.r#type = Some(media_type.to_string());
        asset.title = Some("3D city model data".to_string());
        asset.roles = vec!["data".to_string()];

        if let Some(size) = file_size {
            asset
                .additional_fields
                .insert("file:size".to_string(), Value::Number(size.into()));
            self.uses_file_extension = true;
        }

        self.assets.insert("data".to_string(), asset);
        self
    }

    /// Add a custom asset
    pub fn asset(mut self, key: impl Into<String>, asset: stac::Asset) -> Self {
        self.assets.insert(key.into(), asset);
        self
    }

    /// Add a link
    pub fn link(mut self, link: stac::Link) -> Self {
        self.links.push(link);
        self
    }

    /// Add a self link
    pub fn self_link(mut self, href: impl ToString) -> Self {
        self.links.push(stac::Link::self_(href));
        self
    }

    /// Add a parent link
    pub fn parent_link(mut self, href: impl ToString) -> Self {
        self.links.push(stac::Link::parent(href));
        self
    }

    /// Add a collection link
    pub fn collection_link(mut self, href: impl ToString) -> Self {
        self.links.push(stac::Link::collection(href));
        self
    }

    /// Build the STAC Item
    pub fn build(self) -> Result<stac::Item> {
        let mut item = stac::Item::new(&self.id);

        // Set datetime fields
        item.properties.datetime = self.datetime;
        item.properties.start_datetime = self.start_datetime;
        item.properties.end_datetime = self.end_datetime;
        item.properties.title = self.title;
        item.properties.description = self.description;

        // Extension properties go in properties.additional_fields
        item.properties.additional_fields = self.properties;

        // Set bbox
        if let Some(bbox_vec) = self.bbox {
            item.bbox = bbox_vec.try_into().ok();
        }

        // Set geometry
        if let Some(geom_value) = self.geometry {
            item.geometry = serde_json::from_value(geom_value).ok();
        }

        // Set assets
        item.assets = self.assets;

        // Set links
        item.links = self.links;

        // Build stac_extensions list dynamically
        let mut stac_extensions =
            vec!["https://cityjson.github.io/stac-city3d/v0.1.0/schema.json".to_string()];

        if item.properties.additional_fields.contains_key("proj:code") {
            stac_extensions.push(
                "https://stac-extensions.github.io/projection/v2.0.0/schema.json".to_string(),
            );
        }

        if self.uses_file_extension {
            stac_extensions
                .push("https://stac-extensions.github.io/file/v2.1.0/schema.json".to_string());
        }

        item.extensions = stac_extensions;

        Ok(item)
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
    pub fn from_file(
        file_path: &Path,
        reader: &dyn CityModelMetadataReader,
        base_url: Option<&str>,
        original_url: Option<&str>,
    ) -> Result<Self> {
        Self::from_file_with_crs_override(file_path, reader, base_url, original_url, None)
    }

    /// Helper to create item from file path with an optional CRS override
    pub fn from_file_with_crs_override(
        file_path: &Path,
        reader: &dyn CityModelMetadataReader,
        base_url: Option<&str>,
        original_url: Option<&str>,
        crs_override: Option<&CRS>,
    ) -> Result<Self> {
        let id = file_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        let mut builder = Self::new(id);

        // Set bbox (transformed to WGS84 for STAC compliance)
        if let Ok(bbox) = reader.bbox() {
            let crs = Self::resolve_crs(reader, crs_override);
            let wgs84_bbox = bbox.to_wgs84(&crs)?;
            builder = builder.bbox(wgs84_bbox).geometry_from_bbox();
        }

        // Add CityJSON metadata
        builder = builder.cityjson_metadata(reader)?;

        // Get file size for the asset (File Extension)
        let file_size = std::fs::metadata(file_path).ok().map(|m| m.len());

        // Add data asset - detect ZIP files for proper media type
        let is_zip = file_path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e == "zip")
            .unwrap_or(false);

        let media_type = if is_zip {
            "application/zip"
        } else {
            encoding_media_type(reader.encoding())
        };

        // Generate asset href based on base_url or original_url
        let file_name = file_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("data");

        let href = match base_url {
            Some(base) => {
                let normalized_base = if base.ends_with('/') {
                    base.to_string()
                } else {
                    format!("{base}/")
                };
                format!("{normalized_base}{file_name}")
            }
            None => match original_url {
                Some(url) => url.to_string(),
                None => file_name.to_string(),
            },
        };

        builder = builder.data_asset(href.clone(), media_type, file_size);

        // Add city-model relation link (per STAC 3D City Models Extension)
        builder =
            builder.link(stac::Link::new(&href, "city-model").r#type(Some(media_type.to_string())));

        Ok(builder)
    }

    /// Helper to create item from file path with format suffix in ID
    pub fn from_file_with_format_suffix(
        file_path: &Path,
        reader: &dyn CityModelMetadataReader,
        base_url: Option<&str>,
        original_url: Option<&str>,
    ) -> Result<Self> {
        Self::from_file_with_format_suffix_and_crs(file_path, reader, base_url, original_url, None)
    }

    /// Helper to create item from file path with format suffix and optional CRS override
    pub fn from_file_with_format_suffix_and_crs(
        file_path: &Path,
        reader: &dyn CityModelMetadataReader,
        base_url: Option<&str>,
        original_url: Option<&str>,
        crs_override: Option<&CRS>,
    ) -> Result<Self> {
        let stem = file_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");

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
            let crs = Self::resolve_crs(reader, crs_override);
            let wgs84_bbox = bbox.to_wgs84(&crs)?;
            builder = builder.bbox(wgs84_bbox).geometry_from_bbox();
        }

        // Add CityJSON metadata
        builder = builder.cityjson_metadata(reader)?;

        // Get file size for the asset (File Extension)
        let file_size = std::fs::metadata(file_path).ok().map(|m| m.len());

        let is_zip = file_path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e == "zip")
            .unwrap_or(false);

        let media_type = if is_zip {
            "application/zip"
        } else {
            encoding_media_type(reader.encoding())
        };

        let file_name = file_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("data");

        let href = match base_url {
            Some(base) => {
                let normalized_base = if base.ends_with('/') {
                    base.to_string()
                } else {
                    format!("{base}/")
                };
                format!("{normalized_base}{file_name}")
            }
            None => match original_url {
                Some(url) => url.to_string(),
                None => file_name.to_string(),
            },
        };

        builder = builder.data_asset(href.clone(), media_type, file_size);

        // Add city-model relation link (per STAC 3D City Models Extension)
        builder =
            builder.link(stac::Link::new(&href, "city-model").r#type(Some(media_type.to_string())));

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
        assert_eq!(item.properties.title, Some("Test Item".to_string()));
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

        assert_eq!(
            item.properties
                .additional_fields
                .get("city3d:version")
                .unwrap(),
            "2.0"
        );
        assert_eq!(
            item.properties
                .additional_fields
                .get("city3d:city_objects")
                .unwrap(),
            1
        );
        assert_eq!(
            item.properties.additional_fields.get("proj:code").unwrap(),
            "EPSG:7415"
        );
    }

    #[test]
    fn test_item_builder_from_file() {
        let temp_file = create_test_cityjson();
        let reader = CityJSONReader::new(temp_file.path()).unwrap();

        let builder = StacItemBuilder::from_file(temp_file.path(), &reader, None, None).unwrap();
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
        let geom_value = serde_json::to_value(&item.geometry).unwrap();
        assert_eq!(geom_value["type"], "Polygon");
    }
}
