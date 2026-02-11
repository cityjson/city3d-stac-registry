//! YAML configuration for collection metadata

use crate::error::{CityJsonStacError, Result};
use crate::stac::Provider;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Collection configuration from YAML file
#[derive(Debug, Deserialize, Serialize, Default)]
pub struct CollectionConfigFile {
    /// Collection ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// Collection title
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    /// Collection description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Data license (SPDX identifier)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,

    /// Keywords/tags
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keywords: Option<Vec<String>>,

    /// Providers (organizations that provided/manage data)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub providers: Option<Vec<ProviderConfig>>,

    /// Custom extent (overrides auto-detected)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extent: Option<ExtentConfig>,

    /// Custom summaries (merged with auto-detected)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summaries: Option<HashMap<String, serde_json::Value>>,

    /// Links to add
    #[serde(skip_serializing_if = "Option::is_none")]
    pub links: Option<Vec<LinkConfig>>,

    /// Input paths (files, directories, or glob patterns)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inputs: Option<Vec<String>>,
}

/// Provider configuration from YAML
#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct ProviderConfig {
    /// Provider name
    pub name: String,

    /// Provider URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,

    /// Provider roles (e.g., producer, licensor, processor, host)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub roles: Option<Vec<String>>,

    /// Provider description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl From<ProviderConfig> for Provider {
    fn from(config: ProviderConfig) -> Self {
        Provider {
            name: config.name,
            description: config.description,
            roles: config.roles,
            url: config.url,
        }
    }
}

/// Extent configuration from YAML
#[derive(Debug, Deserialize, Serialize)]
pub struct ExtentConfig {
    /// Spatial extent
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spatial: Option<SpatialExtentConfig>,

    /// Temporal extent
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temporal: Option<TemporalExtentConfig>,
}

/// Spatial extent configuration
#[derive(Debug, Deserialize, Serialize)]
pub struct SpatialExtentConfig {
    /// Bounding box [minx, miny, minz, maxx, maxy, maxz] or [minx, miny, maxx, maxy]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bbox: Option<Vec<f64>>,

    /// Coordinate reference system (e.g., "EPSG:7415")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub crs: Option<String>,
}

/// Temporal extent configuration
#[derive(Debug, Deserialize, Serialize)]
pub struct TemporalExtentConfig {
    /// Start datetime (RFC3339 format)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<String>,

    /// End datetime (RFC3339 format), null for open-ended
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end: Option<String>,
}

/// Link configuration from YAML
#[derive(Debug, Deserialize, Serialize)]
pub struct LinkConfig {
    /// Link relation type
    pub rel: String,

    /// Link href
    pub href: String,

    /// Link type (MIME type)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "type")]
    pub link_type: Option<String>,

    /// Link title
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

impl CollectionConfigFile {
    /// Load config from YAML file
    pub fn from_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = serde_yaml::from_str(&content)
            .map_err(|e| CityJsonStacError::Other(format!("Invalid YAML: {e}")))?;
        Ok(config)
    }

    /// Merge with CLI arguments (CLI takes precedence)
    pub fn merge_with_cli(self, cli_args: &CollectionCliArgs) -> Self {
        CollectionConfigFile {
            id: cli_args.id.clone().or(self.id),
            title: cli_args.title.clone().or(self.title),
            description: cli_args.description.clone().or(self.description),
            license: if cli_args.license.is_some() {
                cli_args.license.clone()
            } else {
                self.license
            },
            keywords: self.keywords,
            providers: self.providers,
            extent: self.extent,
            summaries: self.summaries,
            links: self.links,
            inputs: self.inputs,
        }
    }
}

/// CLI arguments that can override config
#[derive(Debug, Default)]
pub struct CollectionCliArgs {
    pub id: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub license: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_conversion() {
        let config = ProviderConfig {
            name: "Test Provider".to_string(),
            url: Some("https://example.com".to_string()),
            roles: Some(vec!["producer".to_string(), "licensor".to_string()]),
            description: Some("A test provider".to_string()),
        };

        let provider: Provider = config.into();

        assert_eq!(provider.name, "Test Provider");
        assert_eq!(provider.url, Some("https://example.com".to_string()));
        assert_eq!(
            provider.roles,
            Some(vec!["producer".to_string(), "licensor".to_string()])
        );
        assert_eq!(provider.description, Some("A test provider".to_string()));
    }

    #[test]
    fn test_config_merge() {
        let file_config = CollectionConfigFile {
            id: Some("from-file".to_string()),
            title: Some("File Title".to_string()),
            description: Some("File Description".to_string()),
            license: Some("Apache-2.0".to_string()),
            keywords: Some(vec!["tag1".to_string(), "tag2".to_string()]),
            providers: None,
            extent: None,
            summaries: None,
            links: None,
            inputs: None,
        };

        let cli_args = CollectionCliArgs {
            id: Some("from-cli".to_string()),
            title: Some("CLI Title".to_string()),
            description: None,
            license: Some("MIT".to_string()),
        };

        let merged = file_config.merge_with_cli(&cli_args);

        // CLI args should override for id, title, license
        assert_eq!(merged.id, Some("from-cli".to_string()));
        assert_eq!(merged.title, Some("CLI Title".to_string()));
        assert_eq!(merged.license, Some("MIT".to_string()));

        // File config should be preserved for description, keywords
        assert_eq!(merged.description, Some("File Description".to_string()));
        assert_eq!(
            merged.keywords,
            Some(vec!["tag1".to_string(), "tag2".to_string()])
        );
    }
}
