//! STAC data models
//!
//! This file contains STAC type definitions that are derived from
//! [STAC Specification v1.0.0](https://github.com/radiantearth/stac-spec) JSON schemas.
//!
//! The types match the official STAC specification structure with serde
//! annotations for proper JSON serialization/deserialization.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// City object count - either integer or statistics object
///
/// For STAC Items, this is typically a single integer.
/// For STAC Collections, this can be statistics with min/max/total.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CityObjectsCount {
    Integer(u64),
    Statistics { min: u64, max: u64, total: u64 },
}

impl From<u64> for CityObjectsCount {
    fn from(value: u64) -> Self {
        CityObjectsCount::Integer(value)
    }
}

impl From<(u64, u64, u64)> for CityObjectsCount {
    fn from((min, max, total): (u64, u64, u64)) -> Self {
        CityObjectsCount::Statistics { min, max, total }
    }
}

/// STAC Item
///
/// Corresponds to the STAC Item specification:
/// https://github.com/radiantearth/stac-spec/blob/master/item-spec/item-spec.md
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StacItem {
    #[serde(rename = "stac_version")]
    pub stac_version: String,

    #[serde(rename = "stac_extensions")]
    pub stac_extensions: Vec<String>,

    #[serde(rename = "type")]
    pub item_type: String,

    pub id: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub bbox: Option<Vec<f64>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub geometry: Option<Value>,

    pub properties: HashMap<String, Value>,

    pub assets: HashMap<String, Asset>,

    pub links: Vec<Link>,
}

/// STAC Collection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StacCollection {
    #[serde(rename = "stac_version")]
    pub stac_version: String,

    #[serde(rename = "stac_extensions")]
    pub stac_extensions: Vec<String>,

    #[serde(rename = "type")]
    pub collection_type: String,

    pub id: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    pub license: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub keywords: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub providers: Option<Vec<Provider>>,

    pub extent: Extent,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub summaries: Option<HashMap<String, Value>>,

    pub links: Vec<Link>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub assets: Option<HashMap<String, Asset>>,
}

/// STAC Link
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Link {
    pub rel: String,
    pub href: String,

    #[serde(skip_serializing_if = "Option::is_none", rename = "type")]
    pub link_type: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

impl Link {
    pub fn new(rel: impl Into<String>, href: impl Into<String>) -> Self {
        Self {
            rel: rel.into(),
            href: href.into(),
            link_type: None,
            title: None,
        }
    }

    pub fn with_type(mut self, link_type: impl Into<String>) -> Self {
        self.link_type = Some(link_type.into());
        self
    }

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }
}

/// STAC Asset
///
/// Supports File Extension fields (file:size, file:checksum, file:values)
/// https://stac-extensions.github.io/file/v2.1.0/schema.json
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Asset {
    pub href: String,

    #[serde(skip_serializing_if = "Option::is_none", rename = "type")]
    pub media_type: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub roles: Option<Vec<String>>,

    /// File Extension: size of the file in bytes
    #[serde(skip_serializing_if = "Option::is_none", rename = "file:size")]
    pub file_size: Option<u64>,

    /// File Extension: checksum of the file
    #[serde(skip_serializing_if = "Option::is_none", rename = "file:checksum")]
    pub file_checksum: Option<Checksum>,
}

/// File checksum (File Extension)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checksum {
    /// The multihash checksum value
    pub value: String,

    /// The checksum algorithm namespace (e.g., "md5", "sha256")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
}

impl Asset {
    pub fn new(href: impl Into<String>) -> Self {
        Self {
            href: href.into(),
            media_type: None,
            title: None,
            description: None,
            roles: None,
            file_size: None,
            file_checksum: None,
        }
    }

    pub fn with_type(mut self, media_type: impl Into<String>) -> Self {
        self.media_type = Some(media_type.into());
        self
    }

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn with_roles(mut self, roles: Vec<String>) -> Self {
        self.roles = Some(roles);
        self
    }

    /// Set the file size (File Extension)
    pub fn with_file_size(mut self, size: u64) -> Self {
        self.file_size = Some(size);
        self
    }

    /// Set the file checksum (File Extension)
    pub fn with_file_checksum(mut self, checksum: Checksum) -> Self {
        self.file_checksum = Some(checksum);
        self
    }
}

/// STAC Extent
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Extent {
    pub spatial: SpatialExtent,

    pub temporal: TemporalExtent,
}

/// Spatial extent
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SpatialExtent {
    pub bbox: Vec<Vec<f64>>,
}

/// Temporal extent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalExtent {
    pub interval: Vec<Vec<Option<String>>>,
}

impl Default for TemporalExtent {
    fn default() -> Self {
        Self {
            // Default to open-ended interval starting from current time
            interval: vec![vec![Some(chrono::Utc::now().to_rfc3339()), None]],
        }
    }
}

/// Provider information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Provider {
    pub name: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub roles: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_link_creation() {
        let link = Link::new("self", "./item.json")
            .with_type("application/json")
            .with_title("Self link");

        assert_eq!(link.rel, "self");
        assert_eq!(link.href, "./item.json");
        assert_eq!(link.link_type, Some("application/json".to_string()));
        assert_eq!(link.title, Some("Self link".to_string()));
    }

    #[test]
    fn test_asset_creation() {
        let asset = Asset::new("./data.json")
            .with_type("application/json")
            .with_title("Data file")
            .with_roles(vec!["data".to_string()]);

        assert_eq!(asset.href, "./data.json");
        assert_eq!(asset.media_type, Some("application/json".to_string()));
        assert_eq!(asset.title, Some("Data file".to_string()));
        assert_eq!(asset.roles, Some(vec!["data".to_string()]));
    }

    #[test]
    fn test_stac_item_serialization() {
        let mut properties = HashMap::new();
        properties.insert(
            "datetime".to_string(),
            Value::String("2023-01-01T00:00:00Z".to_string()),
        );

        let mut assets = HashMap::new();
        assets.insert("data".to_string(), Asset::new("./data.json"));

        let item = StacItem {
            stac_version: "1.0.0".to_string(),
            stac_extensions: vec![],
            item_type: "Feature".to_string(),
            id: "test-item".to_string(),
            bbox: Some(vec![0.0, 0.0, 0.0, 10.0, 10.0, 10.0]),
            geometry: None,
            properties,
            assets,
            links: vec![Link::new("self", "./item.json")],
        };

        let json = serde_json::to_string(&item).unwrap();
        assert!(json.contains("\"stac_version\":\"1.0.0\""));
        assert!(json.contains("\"id\":\"test-item\""));
    }
}
