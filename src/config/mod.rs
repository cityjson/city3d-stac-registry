//! YAML configuration for collection metadata

use crate::error::{CityJsonStacError, Result};
use crate::stac::Provider;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Input configuration that supports both inline lists and file references
///
/// Supports two formats:
/// 1. Inline list: `inputs: [url1, url2, ...]`
/// 2. File reference: `inputs: {from_file: path/to/urls.txt}`
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum InputsConfig {
    /// Direct list of input paths/URLs
    Inline(Vec<String>),
    /// Reference to a file containing input paths/URLs (one per line)
    FromFile { from_file: String },
}

impl InputsConfig {
    /// Resolve inputs to a list of strings
    ///
    /// For `FromFile` variant, reads the file and returns its lines.
    /// Relative paths in the file are resolved relative to the config file's directory.
    pub fn resolve(&self, config_dir: &Path) -> Result<Vec<String>> {
        match self {
            InputsConfig::Inline(urls) => Ok(urls.clone()),
            InputsConfig::FromFile { from_file } => {
                let file_path = if Path::new(from_file).is_absolute() {
                    PathBuf::from(from_file)
                } else {
                    config_dir.join(from_file)
                };

                let content = std::fs::read_to_string(&file_path).map_err(|e| {
                    CityJsonStacError::Other(format!(
                        "Failed to read inputs file '{}': {}",
                        file_path.display(),
                        e
                    ))
                })?;

                let urls: Vec<String> = content
                    .lines()
                    .filter(|line| !line.trim().is_empty() && !line.trim().starts_with('#'))
                    .map(|s| s.trim().to_string())
                    .collect();

                log::info!(
                    "Loaded {} input URLs from {}",
                    urls.len(),
                    file_path.display()
                );

                Ok(urls)
            }
        }
    }
}

impl Serialize for InputsConfig {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            InputsConfig::Inline(urls) => urls.serialize(serializer),
            InputsConfig::FromFile { from_file } => {
                use serde::ser::SerializeMap;
                let mut map = serializer.serialize_map(Some(1))?;
                map.serialize_entry("from_file", from_file)?;
                map.end()
            }
        }
    }
}

/// Collection configuration from YAML file
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
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
    /// Can be either an inline list or a reference to a file containing URLs
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inputs: Option<InputsConfig>,

    /// Base URL for asset hrefs (e.g., "https://example.com/data/")
    /// If provided, asset hrefs will be absolute URLs
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
}

/// Provider configuration from YAML
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
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
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ExtentConfig {
    /// Spatial extent
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spatial: Option<SpatialExtentConfig>,

    /// Temporal extent
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temporal: Option<TemporalExtentConfig>,
}

/// Spatial extent configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SpatialExtentConfig {
    /// Bounding box [minx, miny, minz, maxx, maxy, maxz] or [minx, miny, maxx, maxy]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bbox: Option<Vec<f64>>,

    /// Coordinate reference system (e.g., "EPSG:7415")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub crs: Option<String>,
}

/// Temporal extent configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TemporalExtentConfig {
    /// Start datetime (RFC3339 format)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<String>,

    /// End datetime (RFC3339 format), null for open-ended
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end: Option<String>,
}

/// Link configuration from YAML
#[derive(Debug, Clone, Deserialize, Serialize)]
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
    /// Load config from YAML or TOML file
    pub fn from_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;

        let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");

        match extension {
            "toml" => toml::from_str(&content)
                .map_err(|e| CityJsonStacError::Other(format!("Invalid TOML: {e}"))),
            _ => serde_yaml::from_str(&content)
                .map_err(|e| CityJsonStacError::Other(format!("Invalid YAML: {e}"))),
        }
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
            base_url: cli_args.base_url.clone().or(self.base_url),
        }
    }
}

/// Catalog configuration from YAML/TOML file
#[derive(Debug, Deserialize, Serialize, Default)]
pub struct CatalogConfigFile {
    /// Catalog ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// Catalog title
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    /// Catalog description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Collections to include in the catalog
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collections: Option<Vec<String>>,

    /// Base URL for catalog child links (applied to all collections)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
}

impl CatalogConfigFile {
    /// Load config from YAML or TOML file
    pub fn from_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;

        let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");

        match extension {
            "toml" => toml::from_str(&content)
                .map_err(|e| CityJsonStacError::Other(format!("Invalid TOML: {e}"))),
            _ => serde_yaml::from_str(&content)
                .map_err(|e| CityJsonStacError::Other(format!("Invalid YAML: {e}"))),
        }
    }

    /// Merge with CLI arguments
    pub fn merge_with_cli(self, cli_args: &CatalogCliArgs) -> Self {
        CatalogConfigFile {
            id: cli_args.id.clone().or(self.id),
            title: cli_args.title.clone().or(self.title),
            description: cli_args.description.clone().or(self.description),
            collections: self.collections,
            base_url: cli_args.base_url.clone().or(self.base_url),
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
    pub base_url: Option<String>,
}

/// CLI arguments that can override catalog config
#[derive(Debug, Default)]
pub struct CatalogCliArgs {
    pub id: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub base_url: Option<String>,
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
            base_url: Some("https://file.example.com/".to_string()),
        };

        let cli_args = CollectionCliArgs {
            id: Some("from-cli".to_string()),
            title: Some("CLI Title".to_string()),
            description: None,
            license: Some("MIT".to_string()),
            base_url: Some("https://cli.example.com/".to_string()),
        };

        let merged = file_config.merge_with_cli(&cli_args);

        // CLI args should override for id, title, license, base_url
        assert_eq!(merged.id, Some("from-cli".to_string()));
        assert_eq!(merged.title, Some("CLI Title".to_string()));
        assert_eq!(merged.license, Some("MIT".to_string()));
        assert_eq!(
            merged.base_url,
            Some("https://cli.example.com/".to_string())
        );

        // File config should be preserved for description, keywords
        assert_eq!(merged.description, Some("File Description".to_string()));
        assert_eq!(
            merged.keywords,
            Some(vec!["tag1".to_string(), "tag2".to_string()])
        );
    }

    #[test]
    fn test_inputs_config_inline() {
        let inputs = InputsConfig::Inline(vec!["file1.json".to_string(), "file2.json".to_string()]);

        let resolved = inputs.resolve(Path::new(".")).unwrap();
        assert_eq!(resolved.len(), 2);
        assert_eq!(resolved[0], "file1.json");
        assert_eq!(resolved[1], "file2.json");
    }

    #[test]
    fn test_inputs_config_from_file() {
        use std::io::Write;

        // Create a temp file with URLs
        let mut temp_file = tempfile::NamedTempFile::new().unwrap();
        writeln!(temp_file, "url1.json").unwrap();
        writeln!(temp_file, "url2.json").unwrap();
        writeln!(temp_file, "# comment").unwrap();
        writeln!(temp_file, "").unwrap(); // empty line
        writeln!(temp_file, "url3.json").unwrap();
        temp_file.flush().unwrap();

        let inputs = InputsConfig::FromFile {
            from_file: temp_file.path().display().to_string(),
        };

        let resolved = inputs.resolve(Path::new(".")).unwrap();
        assert_eq!(resolved.len(), 3);
        assert_eq!(resolved[0], "url1.json");
        assert_eq!(resolved[1], "url2.json");
        assert_eq!(resolved[2], "url3.json");
    }

    #[test]
    fn test_inputs_config_deserialize_inline() {
        let yaml = r#"
- file1.json
- file2.json
"#;
        let inputs: InputsConfig = serde_yaml::from_str(yaml).unwrap();
        match inputs {
            InputsConfig::Inline(urls) => {
                assert_eq!(urls.len(), 2);
            }
            InputsConfig::FromFile { .. } => panic!("Expected Inline"),
        }
    }

    #[test]
    fn test_inputs_config_deserialize_from_file() {
        let yaml = r#"
from_file: /path/to/urls.txt
"#;
        let inputs: InputsConfig = serde_yaml::from_str(yaml).unwrap();
        match inputs {
            InputsConfig::Inline(_) => panic!("Expected FromFile"),
            InputsConfig::FromFile { from_file } => {
                assert_eq!(from_file, "/path/to/urls.txt");
            }
        }
    }
}
