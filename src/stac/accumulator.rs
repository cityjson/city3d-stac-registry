//! Accumulator types for streaming collection generation
//!
//! These types allow accumulating minimal item metadata during streaming processing,
//! without keeping full item JSON in memory.

use crate::metadata::AttributeDefinition;
use crate::stac::CityObjectsCount;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Minimal metadata extracted from a processed item for collection aggregation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemMetadata {
    pub id: String,
    pub bbox: Option<Vec<f64>>,
    pub city3d_version: Option<String>,
    pub city3d_city_objects: Option<CityObjectsCount>,
    pub city3d_lods: Option<Vec<String>>,
    pub city3d_co_types: Option<Vec<String>>,
    pub city3d_attributes: Option<Vec<AttributeDefinition>>,
    pub city3d_semantic_surfaces: Option<bool>,
    pub city3d_textures: Option<bool>,
    pub city3d_materials: Option<bool>,
    /// Projection code (e.g., "EPSG:7415")
    pub proj_code: Option<String>,
}

impl ItemMetadata {
    /// Extract minimal metadata from a stac::Item
    pub fn from_item(item: &stac::Item) -> Self {
        let props = &item.properties.additional_fields;

        let get_string = |key: &str| -> Option<String> {
            props.get(key).and_then(|v| v.as_str().map(String::from))
        };

        let get_bool = |key: &str| -> Option<bool> { props.get(key).and_then(|v| v.as_bool()) };

        let get_string_vec = |key: &str| -> Option<Vec<String>> {
            props.get(key).and_then(|v| {
                v.as_array().map(|arr| {
                    arr.iter()
                        .filter_map(|v| match v {
                            serde_json::Value::String(s) => Some(s.clone()),
                            serde_json::Value::Number(n) => Some(n.to_string()),
                            _ => None,
                        })
                        .collect()
                })
            })
        };

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

        let attributes = props.get("city3d:attributes").and_then(|v| {
            v.as_array().map(|arr| {
                arr.iter()
                    .filter_map(|v| serde_json::from_value(v.clone()).ok())
                    .collect()
            })
        });

        // Convert Bbox enum to Vec<f64> for storage
        let bbox_vec = item.bbox.map(|b| {
            let v: Vec<f64> = b.into();
            v
        });

        Self {
            id: item.id.clone(),
            bbox: bbox_vec,
            city3d_version: get_string("city3d:version"),
            city3d_city_objects: city_objects,
            city3d_lods: get_string_vec("city3d:lods"),
            city3d_co_types: get_string_vec("city3d:co_types"),
            city3d_attributes: attributes,
            city3d_semantic_surfaces: get_bool("city3d:semantic_surfaces"),
            city3d_textures: get_bool("city3d:textures"),
            city3d_materials: get_bool("city3d:materials"),
            proj_code: get_string("proj:code"),
        }
    }

    /// Read metadata from an existing item JSON file on disk
    pub fn from_file(path: &Path) -> Result<Self, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;

        let item: stac::Item = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse {}: {}", path.display(), e))?;

        Ok(Self::from_item(&item))
    }
}

/// Accumulates metadata from multiple items for collection generation.
#[derive(Debug, Clone, Default)]
pub struct CollectionAccumulator {
    pub items_metadata: Vec<ItemMetadata>,
    pub item_links: Vec<(String, Option<String>)>,
    pub errors: Vec<(String, String)>,
}

impl CollectionAccumulator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_item(&mut self, metadata: ItemMetadata, href: String, title: Option<String>) {
        self.items_metadata.push(metadata);
        self.item_links.push((href, title));
    }

    pub fn add_error(&mut self, source: String, error: String) {
        self.errors.push((source, error));
    }

    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    pub fn successful_count(&self) -> usize {
        self.items_metadata.len()
    }

    pub fn error_count(&self) -> usize {
        self.errors.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn create_test_item() -> stac::Item {
        let mut item = stac::Item::new("test-item");
        item.bbox = Some(vec![0.0, 0.0, 0.0, 10.0, 10.0, 10.0].try_into().unwrap());
        item.properties.datetime = Some("2023-01-01T00:00:00Z".parse().unwrap());
        item.properties.additional_fields.insert(
            "city3d:version".to_string(),
            Value::String("2.0".to_string()),
        );
        item.properties
            .additional_fields
            .insert("city3d:city_objects".to_string(), Value::Number(42.into()));
        item.properties.additional_fields.insert(
            "city3d:lods".to_string(),
            Value::Array(vec![
                Value::String("LOD1".to_string()),
                Value::String("LOD2".to_string()),
            ]),
        );
        item.properties.additional_fields.insert(
            "city3d:co_types".to_string(),
            Value::Array(vec![Value::String("Building".to_string())]),
        );
        item.properties
            .additional_fields
            .insert("city3d:textures".to_string(), Value::Bool(true));
        item.properties
            .additional_fields
            .insert("city3d:materials".to_string(), Value::Bool(false));
        item.properties
            .additional_fields
            .insert("city3d:semantic_surfaces".to_string(), Value::Bool(true));

        let asset = stac::Asset::new("./data.json");
        item.assets.insert("data".to_string(), asset);

        item.links.push(stac::Link::self_("./item.json"));

        item
    }

    #[test]
    fn test_item_metadata_from_item() {
        let item = create_test_item();
        let metadata = ItemMetadata::from_item(&item);

        assert_eq!(metadata.id, "test-item");
        assert_eq!(metadata.bbox, Some(vec![0.0, 0.0, 0.0, 10.0, 10.0, 10.0]));
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
