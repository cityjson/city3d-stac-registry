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
    item_assets: Option<HashMap<String, Value>>,
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
            item_assets: None,
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

        // Replace the default temporal with the specified one
        self.extent.temporal = TemporalExtent {
            interval: vec![vec![start_str, end_str]],
        };
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
    ///
    /// Uses the STAC 3D City Models Extension (city3d: prefix)
    /// https://cityjson.github.io/stac-city3d/v0.1.0/schema.json
    pub fn aggregate_cityjson_metadata(
        mut self,
        readers: &[Box<dyn CityModelMetadataReader>],
    ) -> Result<Self> {
        // Collect all versions
        let versions: HashSet<String> = readers.iter().filter_map(|r| r.version().ok()).collect();
        if !versions.is_empty() {
            let version_vec: Vec<String> = versions.into_iter().collect();
            self.summaries.insert(
                "city3d:version".to_string(),
                serde_json::to_value(version_vec)?,
            );
        }

        // Aggregate LODs
        let all_lods: HashSet<String> = readers
            .iter()
            .filter_map(|r| r.lods().ok())
            .flatten()
            .collect();

        if !all_lods.is_empty() {
            let numeric_lods: Vec<Value> = all_lods
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

            self.summaries
                .insert("city3d:lods".to_string(), Value::Array(numeric_lods));
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
                .insert("city3d:co_types".to_string(), serde_json::to_value(types)?);
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

            self.summaries
                .insert("city3d:city_objects".to_string(), stats);
        }

        // Aggregate boolean fields as arrays of unique observed values
        // e.g., [true], [false], or [true, false] per the STAC extension spec
        let semantic_values: HashSet<bool> = readers
            .iter()
            .filter_map(|r| r.semantic_surfaces().ok())
            .collect();
        if !semantic_values.is_empty() {
            let mut vals: Vec<bool> = semantic_values.into_iter().collect();
            vals.sort();
            self.summaries.insert(
                "city3d:semantic_surfaces".to_string(),
                serde_json::to_value(vals)?,
            );
        }

        let texture_values: HashSet<bool> =
            readers.iter().filter_map(|r| r.textures().ok()).collect();
        if !texture_values.is_empty() {
            let mut vals: Vec<bool> = texture_values.into_iter().collect();
            vals.sort();
            self.summaries
                .insert("city3d:textures".to_string(), serde_json::to_value(vals)?);
        }

        let material_values: HashSet<bool> =
            readers.iter().filter_map(|r| r.materials().ok()).collect();
        if !material_values.is_empty() {
            let mut vals: Vec<bool> = material_values.into_iter().collect();
            vals.sort();
            self.summaries
                .insert("city3d:materials".to_string(), serde_json::to_value(vals)?);
        }

        // Aggregate proj:code (array of strings, e.g. ["EPSG:7415", "EPSG:28992"])
        let unique_proj_codes: HashSet<String> = readers
            .iter()
            .filter_map(|r| r.crs().ok())
            .filter_map(|crs| crs.to_stac_proj_code())
            .collect();

        if !unique_proj_codes.is_empty() {
            let mut codes: Vec<String> = unique_proj_codes.into_iter().collect();
            codes.sort();
            self.summaries
                .insert("proj:code".to_string(), serde_json::to_value(codes)?);
        }

        // Merge all bounding boxes for spatial extent (transformed to WGS84)
        let bboxes: Vec<BBox3D> = readers
            .iter()
            .filter_map(|r| {
                let bbox = r.bbox().ok()?;
                let crs = r.crs().unwrap_or_default();
                bbox.to_wgs84(&crs).ok()
            })
            .collect();

        if !bboxes.is_empty() {
            let mut merged = bboxes[0].clone();
            for bbox in &bboxes[1..] {
                merged = merged.merge(bbox);
            }
            self = self.spatial_extent(merged);
        }

        Ok(self)
    }

    /// Aggregate 3D City Models metadata from ItemMetadata
    ///
    /// This is the streaming-friendly version that accepts pre-extracted ItemMetadata
    /// instead of full StacItem objects, reducing memory usage during collection generation.
    ///
    /// Uses the STAC 3D City Models Extension (city3d: prefix)
    /// https://cityjson.github.io/stac-city3d/v0.1.0/schema.json
    pub fn aggregate_from_metadata(
        mut self,
        items_metadata: &[crate::stac::ItemMetadata],
    ) -> Result<Self> {
        use crate::stac::CityObjectsCount;

        // Collect all versions
        let versions: HashSet<String> = items_metadata
            .iter()
            .filter_map(|m| m.city3d_version.clone())
            .collect();
        if !versions.is_empty() {
            let version_vec: Vec<String> = versions.into_iter().collect();
            self.summaries.insert(
                "city3d:version".to_string(),
                serde_json::to_value(version_vec)?,
            );
        }

        // Aggregate LODs (preserve as Values to handle both string and numeric)
        let mut unique_lods: HashSet<String> = HashSet::new();
        let mut lod_values: Vec<Value> = Vec::new();

        for metadata in items_metadata {
            if let Some(lods) = &metadata.city3d_lods {
                for lod in lods {
                    if !unique_lods.contains(lod) {
                        unique_lods.insert(lod.clone());
                        // Try to parse as number, fall back to string
                        if let Ok(num) = lod.parse::<f64>() {
                            if let Some(n) = serde_json::Number::from_f64(num) {
                                lod_values.push(Value::Number(n));
                            } else {
                                lod_values.push(Value::String(lod.clone()));
                            }
                        } else {
                            lod_values.push(Value::String(lod.clone()));
                        }
                    }
                }
            }
        }

        if !lod_values.is_empty() {
            self.summaries
                .insert("city3d:lods".to_string(), Value::Array(lod_values));
        }

        // Aggregate city object types
        let all_types: HashSet<String> = items_metadata
            .iter()
            .filter_map(|m| m.city3d_co_types.clone())
            .flatten()
            .collect();
        if !all_types.is_empty() {
            let mut types: Vec<String> = all_types.into_iter().collect();
            types.sort();
            self.summaries
                .insert("city3d:co_types".to_string(), serde_json::to_value(types)?);
        }

        // City object count statistics
        let counts: Vec<u64> = items_metadata
            .iter()
            .filter_map(|m| match &m.city3d_city_objects {
                Some(CityObjectsCount::Integer(n)) => Some(*n),
                Some(CityObjectsCount::Statistics { total, .. }) => Some(*total),
                None => None,
            })
            .collect();

        if !counts.is_empty() {
            let min = *counts.iter().min().unwrap();
            let max = *counts.iter().max().unwrap();
            let total: u64 = counts.iter().sum();

            let stats = serde_json::json!({
                "min": min,
                "max": max,
                "total": total
            });

            self.summaries
                .insert("city3d:city_objects".to_string(), stats);
        }

        // Aggregate boolean fields as arrays of unique observed values
        let semantic_values: HashSet<bool> = items_metadata
            .iter()
            .filter_map(|m| m.city3d_semantic_surfaces)
            .collect();
        if !semantic_values.is_empty() {
            let mut vals: Vec<bool> = semantic_values.into_iter().collect();
            vals.sort();
            self.summaries.insert(
                "city3d:semantic_surfaces".to_string(),
                serde_json::to_value(vals)?,
            );
        }

        let texture_values: HashSet<bool> = items_metadata
            .iter()
            .filter_map(|m| m.city3d_textures)
            .collect();
        if !texture_values.is_empty() {
            let mut vals: Vec<bool> = texture_values.into_iter().collect();
            vals.sort();
            self.summaries
                .insert("city3d:textures".to_string(), serde_json::to_value(vals)?);
        }

        let material_values: HashSet<bool> = items_metadata
            .iter()
            .filter_map(|m| m.city3d_materials)
            .collect();
        if !material_values.is_empty() {
            let mut vals: Vec<bool> = material_values.into_iter().collect();
            vals.sort();
            self.summaries
                .insert("city3d:materials".to_string(), serde_json::to_value(vals)?);
        }

        // Merge spatial extents from item bboxes
        let bboxes: Vec<&Vec<f64>> = items_metadata
            .iter()
            .filter_map(|m| m.bbox.as_ref())
            .collect();

        if !bboxes.is_empty() {
            // Parse bbox into BBox3D (handle both 4-element and 6-element bboxes)
            let parsed_bboxes: Vec<BBox3D> = bboxes
                .iter()
                .filter_map(|bbox| {
                    if bbox.len() == 6 {
                        Some(BBox3D::new(
                            (*bbox)[0],
                            (*bbox)[1],
                            (*bbox)[2],
                            (*bbox)[3],
                            (*bbox)[4],
                            (*bbox)[5],
                        ))
                    } else if bbox.len() >= 4 {
                        // 2D bbox - use 0.0 for z values
                        Some(BBox3D::new(
                            (*bbox)[0],
                            (*bbox)[1],
                            0.0,
                            (*bbox)[2],
                            (*bbox)[3],
                            0.0,
                        ))
                    } else {
                        None
                    }
                })
                .collect();

            if !parsed_bboxes.is_empty() {
                let mut merged = parsed_bboxes[0].clone();
                for bbox in &parsed_bboxes[1..] {
                    merged = merged.merge(bbox);
                }
                self = self.spatial_extent(merged);
            }
        }

        Ok(self)
    }

    /// Aggregate 3D City Models metadata from pre-parsed STAC items
    ///
    /// This method is useful when STAC items were generated separately (e.g., for assets
    /// stored in Object Storage) and need to be aggregated into a collection.
    /// It extracts 3D City Models extension properties (city3d:*) from item properties and merges them.
    ///
    /// Uses the STAC 3D City Models Extension (city3d: prefix)
    /// https://cityjson.github.io/stac-city3d/v0.1.0/schema.json
    pub fn aggregate_from_items(mut self, items: &[crate::stac::models::StacItem]) -> Result<Self> {
        use crate::stac::models::StacItem;
        use serde_json::Value;

        // Helper to extract string array from item properties
        fn get_string_array(item: &StacItem, key: &str) -> Vec<String> {
            item.properties
                .get(key)
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default()
        }

        // Helper to extract number array/mixed array from items
        fn get_lod_array(item: &StacItem) -> Vec<Value> {
            if let Some(arr) = item
                .properties
                .get("city3d:lods")
                .and_then(|v| v.as_array())
            {
                arr.clone()
            } else {
                Vec::new()
            }
        }

        // Helper to extract string from item properties
        fn get_string(item: &StacItem, key: &str) -> Option<String> {
            item.properties
                .get(key)
                .and_then(|v| v.as_str())
                .map(String::from)
        }

        // Helper to extract integer from item properties
        fn get_int(item: &StacItem, key: &str) -> Option<i64> {
            item.properties.get(key).and_then(|v| v.as_i64())
        }

        // Collect all versions
        let versions: HashSet<String> = items
            .iter()
            .filter_map(|item| get_string(item, "city3d:version"))
            .collect();
        if !versions.is_empty() {
            let version_vec: Vec<String> = versions.into_iter().collect();
            self.summaries.insert(
                "city3d:version".to_string(),
                serde_json::to_value(version_vec)?,
            );
        }

        // Aggregate LODs
        // Since they are now Values (Numbers), we collect unique Values by stringifying them first
        let mut unique_lods: HashSet<String> = HashSet::new();
        let mut lod_values: Vec<Value> = Vec::new();

        for item in items {
            for lod in get_lod_array(item) {
                let s = lod.to_string();
                if !unique_lods.contains(&s) {
                    unique_lods.insert(s);
                    lod_values.push(lod);
                }
            }
        }

        if !lod_values.is_empty() {
            self.summaries
                .insert("city3d:lods".to_string(), Value::Array(lod_values));
        }

        // Aggregate city object types
        let all_types: HashSet<String> = items
            .iter()
            .flat_map(|item| get_string_array(item, "city3d:co_types"))
            .collect();
        if !all_types.is_empty() {
            let mut types: Vec<String> = all_types.into_iter().collect();
            types.sort();
            self.summaries
                .insert("city3d:co_types".to_string(), serde_json::to_value(types)?);
        }

        // City object count statistics
        let counts: Vec<i64> = items
            .iter()
            .filter_map(|item| get_int(item, "city3d:city_objects"))
            .collect();
        if !counts.is_empty() {
            let min = *counts.iter().min().unwrap();
            let max = *counts.iter().max().unwrap();
            let total: i64 = counts.iter().sum();

            let stats = serde_json::json!({
                "min": min,
                "max": max,
                "total": total
            });

            self.summaries
                .insert("city3d:city_objects".to_string(), stats);
        }

        // Aggregate boolean fields as arrays of unique observed values
        let semantic_values: HashSet<bool> = items
            .iter()
            .filter_map(|item| {
                item.properties
                    .get("city3d:semantic_surfaces")
                    .and_then(|v| v.as_bool())
            })
            .collect();
        if !semantic_values.is_empty() {
            let mut vals: Vec<bool> = semantic_values.into_iter().collect();
            vals.sort();
            self.summaries.insert(
                "city3d:semantic_surfaces".to_string(),
                serde_json::to_value(vals)?,
            );
        }

        let texture_values: HashSet<bool> = items
            .iter()
            .filter_map(|item| {
                item.properties
                    .get("city3d:textures")
                    .and_then(|v| v.as_bool())
            })
            .collect();
        if !texture_values.is_empty() {
            let mut vals: Vec<bool> = texture_values.into_iter().collect();
            vals.sort();
            self.summaries
                .insert("city3d:textures".to_string(), serde_json::to_value(vals)?);
        }

        let material_values: HashSet<bool> = items
            .iter()
            .filter_map(|item| {
                item.properties
                    .get("city3d:materials")
                    .and_then(|v| v.as_bool())
            })
            .collect();
        if !material_values.is_empty() {
            let mut vals: Vec<bool> = material_values.into_iter().collect();
            vals.sort();
            self.summaries
                .insert("city3d:materials".to_string(), serde_json::to_value(vals)?);
        }

        // Aggregate proj:code (array of strings)
        let unique_proj_codes: HashSet<String> = items
            .iter()
            .filter_map(|item| get_string(item, "proj:code"))
            .collect();

        if !unique_proj_codes.is_empty() {
            let mut codes: Vec<String> = unique_proj_codes.into_iter().collect();
            codes.sort();
            self.summaries
                .insert("proj:code".to_string(), serde_json::to_value(codes)?);
        }

        // Merge spatial extents from item bboxes
        let bboxes: Vec<Vec<f64>> = items.iter().filter_map(|item| item.bbox.clone()).collect();

        if !bboxes.is_empty() {
            // Parse bbox into BBox3D (handle both 4-element and 6-element bboxes)
            let parsed_bboxes: Vec<BBox3D> = bboxes
                .iter()
                .filter_map(|bbox| {
                    if bbox.len() == 6 {
                        Some(BBox3D::new(
                            bbox[0], bbox[1], bbox[2], bbox[3], bbox[4], bbox[5],
                        ))
                    } else if bbox.len() >= 4 {
                        // 2D bbox - use 0.0 for z values
                        Some(BBox3D::new(bbox[0], bbox[1], 0.0, bbox[2], bbox[3], 0.0))
                    } else {
                        None
                    }
                })
                .collect();

            if !parsed_bboxes.is_empty() {
                let mut merged = parsed_bboxes[0].clone();
                for bbox in &parsed_bboxes[1..] {
                    merged = merged.merge(bbox);
                }
                self = self.spatial_extent(merged);
            }
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

        // Build stac_extensions list dynamically based on which extensions are used
        let mut stac_extensions =
            vec!["https://cityjson.github.io/stac-city3d/v0.1.0/schema.json".to_string()];

        // Add Projection Extension if proj:code is in summaries
        if self.summaries.contains_key("proj:code") {
            stac_extensions.push(
                "https://stac-extensions.github.io/projection/v2.0.0/schema.json".to_string(),
            );
        }

        // Add Stats Extension if we have statistics (min/max for city_objects)
        if self.summaries.contains_key("city3d:city_objects") {
            stac_extensions
                .push("https://stac-extensions.github.io/stats/v0.2.0/schema.json".to_string());
        }

        // Auto-generate item_assets if not explicitly set and we have city3d summaries
        let item_assets = if self.item_assets.is_some() {
            stac_extensions.push(
                "https://stac-extensions.github.io/item-assets/v1.1.0/schema.json".to_string(),
            );
            self.item_assets
        } else if self.summaries.contains_key("city3d:version") {
            // Auto-populate a default data asset template for city model collections
            let mut assets = HashMap::new();
            assets.insert(
                "data".to_string(),
                serde_json::json!({
                    "title": "3D City Model data file",
                    "roles": ["data"]
                }),
            );
            stac_extensions.push(
                "https://stac-extensions.github.io/item-assets/v1.1.0/schema.json".to_string(),
            );
            Some(assets)
        } else {
            None
        };

        // Add File Extension if any item_assets or assets reference file:size
        stac_extensions
            .push("https://stac-extensions.github.io/file/v2.1.0/schema.json".to_string());

        Ok(StacCollection {
            stac_version: "1.1.0".to_string(),
            stac_extensions,
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
            item_assets,
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
            "transform": {{
                "scale": [0.01, 0.01, 0.01],
                "translate": [100000, 200000, 0]
            }},
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
                        "boundaries": [[[[0,0,0]]]]
                    }}]
                }}
            }},
            "vertices": [[0,0,0]]
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
        let lods = summaries.get("city3d:lods").unwrap().as_array().unwrap();
        assert_eq!(lods.len(), 2);

        // Check aggregated types
        let types = summaries
            .get("city3d:co_types")
            .unwrap()
            .as_array()
            .unwrap();
        assert_eq!(types.len(), 2);

        // Check city object stats
        let stats = summaries.get("city3d:city_objects").unwrap();
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

        assert!(!collection.extent.temporal.interval.is_empty());
        assert_eq!(collection.extent.temporal.interval.len(), 1);
    }
}
