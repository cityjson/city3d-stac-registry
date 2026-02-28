//! Accumulator types for streaming collection generation
//!
//! These types allow accumulating minimal item metadata during streaming processing,
//! without keeping full item JSON in memory.

use crate::metadata::AttributeDefinition;
use crate::stac::{CityObjectsCount, StacItem};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Minimal metadata extracted from a processed item for collection aggregation.
///
/// This struct holds only the essential fields needed to build a collection,
/// avoiding the need to keep full StacItem objects in memory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemMetadata {
    /// Item ID
    pub id: String,

    /// Bounding box [min_x, min_y, min_z, max_x, max_y, max_z] or 2D variant
    pub bbox: Option<Vec<f64>>,

    /// Format encoding (e.g., "CityJSON", "CityGML")
    pub city3d_encoding: Option<String>,

    /// Specification version (e.g., "2.0")
    pub city3d_version: Option<String>,

    /// City objects count
    pub city3d_city_objects: Option<CityObjectsCount>,

    /// Levels of detail present
    pub city3d_lods: Option<Vec<String>>,

    /// City object types present
    pub city3d_co_types: Option<Vec<String>>,

    /// Attribute definitions
    pub city3d_attributes: Option<Vec<AttributeDefinition>>,

    /// Whether semantic surfaces are present
    pub city3d_semantic_surfaces: Option<bool>,

    /// Whether textures are present
    pub city3d_textures: Option<bool>,

    /// Whether materials are present
    pub city3d_materials: Option<bool>,
}

impl ItemMetadata {
    /// Extract minimal metadata from a StacItem
    pub fn from_item(item: &StacItem) -> Self {
        let props = &item.properties;

        // Helper to extract string property
        let get_string = |key: &str| -> Option<String> {
            props.get(key).and_then(|v| v.as_str().map(String::from))
        };

        // Helper to extract bool property
        let get_bool = |key: &str| -> Option<bool> { props.get(key).and_then(|v| v.as_bool()) };

        // Helper to extract vec of strings
        let get_string_vec = |key: &str| -> Option<Vec<String>> {
            props.get(key).and_then(|v| {
                v.as_array().map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
            })
        };

        // Extract city_objects (could be int or stats object)
        let city_objects = props.get("city3d:city_objects").and_then(|v| {
            if let Some(n) = v.as_u64() {
                Some(CityObjectsCount::Integer(n))
            } else if let Some(obj) = v.as_object() {
                let min = obj.get("min").and_then(|v| v.as_u64())?;
                let max = obj.get("max").and_then(|v| v.as_u64())?;
                let total = obj.get("total").and_then(|v| v.as_u64())?;
                Some(CityObjectsCount::Statistics { min, max, total })
            } else {
                None
            }
        });

        // Extract attributes array
        let attributes = props.get("city3d:attributes").and_then(|v| {
            v.as_array().map(|arr| {
                arr.iter()
                    .filter_map(|v| serde_json::from_value(v.clone()).ok())
                    .collect()
            })
        });

        Self {
            id: item.id.clone(),
            bbox: item.bbox.clone(),
            city3d_encoding: get_string("city3d:encoding"),
            city3d_version: get_string("city3d:version"),
            city3d_city_objects: city_objects,
            city3d_lods: get_string_vec("city3d:lods"),
            city3d_co_types: get_string_vec("city3d:co_types"),
            city3d_attributes: attributes,
            city3d_semantic_surfaces: get_bool("city3d:semantic_surfaces"),
            city3d_textures: get_bool("city3d:textures"),
            city3d_materials: get_bool("city3d:materials"),
        }
    }

    /// Read metadata from an existing item JSON file on disk
    pub fn from_file(path: &Path) -> Result<Self, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;

        let item: StacItem = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse {}: {}", path.display(), e))?;

        Ok(Self::from_item(&item))
    }
}

/// Accumulates metadata from multiple items for collection generation.
///
/// This struct is used during streaming processing to collect metadata
/// from all items (both newly processed and existing on disk).
#[derive(Debug, Clone, Default)]
pub struct CollectionAccumulator {
    /// Metadata from all processed/skipped items
    pub items_metadata: Vec<ItemMetadata>,

    /// Item links for the collection (href, title)
    pub item_links: Vec<(String, Option<String>)>,

    /// Processing errors (source, error_message)
    pub errors: Vec<(String, String)>,
}

impl CollectionAccumulator {
    /// Create a new empty accumulator
    pub fn new() -> Self {
        Self::default()
    }

    /// Add metadata from a newly processed item
    pub fn add_item(&mut self, metadata: ItemMetadata, href: String, title: Option<String>) {
        self.items_metadata.push(metadata);
        self.item_links.push((href, title));
    }

    /// Add an error
    pub fn add_error(&mut self, source: String, error: String) {
        self.errors.push((source, error));
    }

    /// Check if there were any errors
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Get the number of successfully processed items
    pub fn successful_count(&self) -> usize {
        self.items_metadata.len()
    }

    /// Get the number of errors
    pub fn error_count(&self) -> usize {
        self.errors.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stac::{Asset, Link};
    use serde_json::Value;
    use std::collections::HashMap;

    fn create_test_item() -> StacItem {
        let mut properties = HashMap::new();
        properties.insert(
            "datetime".to_string(),
            Value::String("2023-01-01T00:00:00Z".to_string()),
        );
        properties.insert(
            "city3d:encoding".to_string(),
            Value::String("CityJSON".to_string()),
        );
        properties.insert(
            "city3d:version".to_string(),
            Value::String("2.0".to_string()),
        );
        properties.insert("city3d:city_objects".to_string(), Value::Number(42.into()));
        properties.insert(
            "city3d:lods".to_string(),
            Value::Array(vec![
                Value::String("LOD1".to_string()),
                Value::String("LOD2".to_string()),
            ]),
        );
        properties.insert(
            "city3d:co_types".to_string(),
            Value::Array(vec![Value::String("Building".to_string())]),
        );
        properties.insert("city3d:textures".to_string(), Value::Bool(true));
        properties.insert("city3d:materials".to_string(), Value::Bool(false));
        properties.insert("city3d:semantic_surfaces".to_string(), Value::Bool(true));

        let mut assets = HashMap::new();
        assets.insert("data".to_string(), Asset::new("./data.json"));

        StacItem {
            stac_version: "1.1.0".to_string(),
            stac_extensions: vec![],
            item_type: "Feature".to_string(),
            id: "test-item".to_string(),
            bbox: Some(vec![0.0, 0.0, 0.0, 10.0, 10.0, 10.0]),
            geometry: None,
            properties,
            assets,
            links: vec![Link::new("self", "./item.json")],
        }
    }

    #[test]
    fn test_item_metadata_from_item() {
        let item = create_test_item();
        let metadata = ItemMetadata::from_item(&item);

        assert_eq!(metadata.id, "test-item");
        assert_eq!(metadata.bbox, Some(vec![0.0, 0.0, 0.0, 10.0, 10.0, 10.0]));
        assert_eq!(metadata.city3d_encoding, Some("CityJSON".to_string()));
        assert_eq!(metadata.city3d_version, Some("2.0".to_string()));
        assert_eq!(
            metadata.city3d_city_objects,
            Some(CityObjectsCount::Integer(42))
        );
        assert_eq!(
            metadata.city3d_lods,
            Some(vec!["LOD1".to_string(), "LOD2".to_string()])
        );
        assert_eq!(metadata.city3d_co_types, Some(vec!["Building".to_string()]));
        assert_eq!(metadata.city3d_textures, Some(true));
        assert_eq!(metadata.city3d_materials, Some(false));
        assert_eq!(metadata.city3d_semantic_surfaces, Some(true));
    }

    #[test]
    fn test_collection_accumulator() {
        let mut accumulator = CollectionAccumulator::new();

        let item = create_test_item();
        let metadata = ItemMetadata::from_item(&item);

        accumulator.add_item(
            metadata,
            "./items/test-item.json".to_string(),
            Some("test-item".to_string()),
        );

        assert_eq!(accumulator.successful_count(), 1);
        assert_eq!(accumulator.error_count(), 0);
        assert!(!accumulator.has_errors());

        accumulator.add_error("failed.json".to_string(), "Parse error".to_string());

        assert_eq!(accumulator.successful_count(), 1);
        assert_eq!(accumulator.error_count(), 1);
        assert!(accumulator.has_errors());
    }
}
