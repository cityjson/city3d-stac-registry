//! STAC Collection builder

use crate::error::{CityJsonStacError, Result};
use crate::metadata::BBox3D;
use crate::reader::CityModelMetadataReader;
use crate::stac::models::{Asset, Extent, Link, Provider, StacCollection, TemporalExtent};
use chrono::{DateTime, Utc};
use serde_json::Value;
use std::collections::{HashMap, HashSet};

/// Builder for STAC Collections
pub struct StacCollectionBuilder {
    id: String,
    title: Option<String>,
    description: Option<String>,
    license: String,
    keywords: Option<Vec<String>>,
    providers: Option<Vec<Provider>>,
    extent: Extent,
    summaries: HashMap<String, Value>,
    links: Vec<Link>,
    assets: HashMap<String, Asset>,
}

impl StacCollectionBuilder {
    /// Create a new STAC Collection builder
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: None,
            description: None,
            license: "proprietary".to_string(),
            keywords: None,
            providers: None,
            extent: Extent::default(),
            summaries: HashMap::new(),
            links: Vec::new(),
            assets: HashMap::new(),
        }
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

    /// Set license
    pub fn license(mut self, license: impl Into<String>) -> Self {
        self.license = license.into();
        self
    }

    /// Set keywords
    pub fn keywords(mut self, keywords: Vec<String>) -> Self {
        self.keywords = Some(keywords);
        self
    }

    /// Add a provider
    pub fn provider(mut self, provider: Provider) -> Self {
        self.providers.get_or_insert_with(Vec::new).push(provider);
        self
    }

    /// Set spatial extent from bounding box
    pub fn spatial_extent(mut self, bbox: BBox3D) -> Self {
        self.extent.spatial.bbox.push(bbox.to_array().to_vec());
        self
    }

    /// Set temporal extent
    pub fn temporal_extent(
        mut self,
        start: Option<DateTime<Utc>>,
        end: Option<DateTime<Utc>>,
    ) -> Self {
        let start_str = start.map(|dt| dt.to_rfc3339());
        let end_str = end.map(|dt| dt.to_rfc3339());

        let temporal = self
            .extent
            .temporal
            .get_or_insert_with(TemporalExtent::default);

        temporal.interval.push(vec![start_str, end_str]);
        self
    }

    /// Add a summary property
    pub fn summary(mut self, key: impl Into<String>, value: Value) -> Self {
        self.summaries.insert(key.into(), value);
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

    /// Add an item link
    pub fn item_link(mut self, href: impl Into<String>, title: Option<String>) -> Self {
        let mut link = Link::new("item", href).with_type("application/json");

        if let Some(t) = title {
            link = link.with_title(t);
        }

        self.links.push(link);
        self
    }

    /// Add an asset
    pub fn asset(mut self, key: impl Into<String>, asset: Asset) -> Self {
        self.assets.insert(key.into(), asset);
        self
    }

    /// Aggregate CityJSON metadata from multiple readers
    pub fn aggregate_cityjson_metadata(
        mut self,
        readers: &[Box<dyn CityModelMetadataReader>],
    ) -> Result<Self> {
        // Collect all encodings
        let encodings: HashSet<String> = readers.iter().map(|r| r.encoding().to_string()).collect();
        let encoding_vec: Vec<String> = encodings.into_iter().collect();
        self.summaries.insert(
            "cj:encoding".to_string(),
            serde_json::to_value(encoding_vec)?,
        );

        // Collect all versions
        let versions: HashSet<String> = readers.iter().filter_map(|r| r.version().ok()).collect();
        if !versions.is_empty() {
            let version_vec: Vec<String> = versions.into_iter().collect();
            self.summaries
                .insert("cj:version".to_string(), serde_json::to_value(version_vec)?);
        }

        // Aggregate LODs
        let all_lods: HashSet<String> = readers
            .iter()
            .filter_map(|r| r.lods().ok())
            .flatten()
            .collect();

        if !all_lods.is_empty() {
            let mut lods: Vec<String> = all_lods.into_iter().collect();
            lods.sort();
            self.summaries
                .insert("cj:lods".to_string(), serde_json::to_value(lods)?);
        }

        // Aggregate city object types
        let all_types: HashSet<String> = readers
            .iter()
            .filter_map(|r| r.city_object_types().ok())
            .flatten()
            .collect();

        if !all_types.is_empty() {
            let mut types: Vec<String> = all_types.into_iter().collect();
            types.sort();
            self.summaries
                .insert("cj:co_types".to_string(), serde_json::to_value(types)?);
        }

        // City object count statistics
        let counts: Vec<usize> = readers
            .iter()
            .filter_map(|r| r.city_object_count().ok())
            .collect();

        if !counts.is_empty() {
            let min = *counts.iter().min().unwrap();
            let max = *counts.iter().max().unwrap();
            let total: usize = counts.iter().sum();

            let stats = serde_json::json!({
                "min": min,
                "max": max,
                "total": total
            });

            self.summaries.insert("cj:city_objects".to_string(), stats);
        }

        // Aggregate EPSG codes
        let epsg_codes: HashSet<u32> = readers
            .iter()
            .filter_map(|r| r.crs().ok())
            .filter_map(|crs| crs.to_stac_epsg())
            .collect();

        if !epsg_codes.is_empty() {
            let codes: Vec<u32> = epsg_codes.into_iter().collect();
            self.summaries
                .insert("proj:epsg".to_string(), serde_json::to_value(codes)?);
        }

        // Merge all bounding boxes for spatial extent
        let bboxes: Vec<BBox3D> = readers.iter().filter_map(|r| r.bbox().ok()).collect();

        if !bboxes.is_empty() {
            let mut merged = bboxes[0].clone();
            for bbox in &bboxes[1..] {
                merged = merged.merge(bbox);
            }
            self = self.spatial_extent(merged);
        }

        Ok(self)
    }

    /// Build the STAC Collection
    pub fn build(self) -> Result<StacCollection> {
        // Validate spatial extent
        if self.extent.spatial.bbox.is_empty() {
            return Err(CityJsonStacError::StacError(
                "Spatial extent bbox is required".to_string(),
            ));
        }

        Ok(StacCollection {
            stac_version: "1.0.0".to_string(),
            stac_extensions: vec![
                "https://raw.githubusercontent.com/cityjson/cityjson-stac/main/stac-extension/schema.json".to_string(),
                "https://stac-extensions.github.io/projection/v1.1.0/schema.json".to_string(),
            ],
            collection_type: "Collection".to_string(),
            id: self.id,
            title: self.title,
            description: self.description,
            license: self.license,
            keywords: self.keywords,
            providers: self.providers,
            extent: self.extent,
            summaries: if self.summaries.is_empty() {
                None
            } else {
                Some(self.summaries)
            },
            links: self.links,
            assets: if self.assets.is_empty() {
                None
            } else {
                Some(self.assets)
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reader::CityJSONReader;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn create_test_cityjson(version: &str, lod: &str, obj_type: &str) -> NamedTempFile {
        let mut temp_file = NamedTempFile::new().unwrap();
        let cityjson = format!(
            r#"{{
            "type": "CityJSON",
            "version": "{}",
            "metadata": {{
                "geographicalExtent": [1.0, 2.0, 0.0, 10.0, 20.0, 30.0],
                "referenceSystem": "https://www.opengis.net/def/crs/EPSG/0/7415"
            }},
            "CityObjects": {{
                "obj1": {{
                    "type": "{}",
                    "geometry": [{{
                        "type": "Solid",
                        "lod": "{}",
                        "boundaries": []
                    }}]
                }}
            }},
            "vertices": []
        }}"#,
            version, obj_type, lod
        );

        writeln!(temp_file, "{}", cityjson).unwrap();
        temp_file.flush().unwrap();
        temp_file
    }

    #[test]
    fn test_collection_builder_basic() {
        let bbox = BBox3D::new(0.0, 0.0, 0.0, 10.0, 10.0, 10.0);

        let collection = StacCollectionBuilder::new("test-collection")
            .title("Test Collection")
            .description("A test collection")
            .license("CC-BY-4.0")
            .spatial_extent(bbox)
            .build()
            .unwrap();

        assert_eq!(collection.id, "test-collection");
        assert_eq!(collection.title, Some("Test Collection".to_string()));
        assert_eq!(collection.license, "CC-BY-4.0");
        assert!(!collection.extent.spatial.bbox.is_empty());
    }

    #[test]
    fn test_collection_aggregate_metadata() {
        let file1 = create_test_cityjson("2.0", "2", "Building");
        let file2 = create_test_cityjson("2.0", "3", "Road");

        let reader1 = CityJSONReader::new(file1.path()).unwrap();
        let reader2 = CityJSONReader::new(file2.path()).unwrap();

        let readers: Vec<Box<dyn CityModelMetadataReader>> =
            vec![Box::new(reader1), Box::new(reader2)];

        let collection = StacCollectionBuilder::new("test")
            .aggregate_cityjson_metadata(&readers)
            .unwrap()
            .build()
            .unwrap();

        let summaries = collection.summaries.unwrap();

        // Check aggregated LODs
        let lods = summaries.get("cj:lods").unwrap().as_array().unwrap();
        assert_eq!(lods.len(), 2);

        // Check aggregated types
        let types = summaries.get("cj:co_types").unwrap().as_array().unwrap();
        assert_eq!(types.len(), 2);

        // Check city object stats
        let stats = summaries.get("cj:city_objects").unwrap();
        assert_eq!(stats["total"], 2);
        assert_eq!(stats["min"], 1);
        assert_eq!(stats["max"], 1);
    }

    #[test]
    fn test_collection_temporal_extent() {
        let bbox = BBox3D::new(0.0, 0.0, 0.0, 10.0, 10.0, 10.0);
        let start = Utc::now();

        let collection = StacCollectionBuilder::new("test")
            .spatial_extent(bbox)
            .temporal_extent(Some(start), None)
            .build()
            .unwrap();

        assert!(collection.extent.temporal.is_some());
        let temporal = collection.extent.temporal.unwrap();
        assert_eq!(temporal.interval.len(), 1);
    }
}
