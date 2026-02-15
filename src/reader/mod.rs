//! Reader implementations for different CityJSON formats
//!
//! This module provides a unified approach to reading CityJSON files
//! from both local filesystem and remote storage (HTTP, S3, Azure, GCS)
//! using the object_store crate.
//!
pub mod cityjson;
pub mod cjseq;
pub mod fcb;

pub use cityjson::CityJSONReader;
pub use cjseq::CityJSONSeqReader;
pub use fcb::FlatCityBufReader;

use crate::error::{CityJsonStacError, Result};
use crate::metadata::{AttributeDefinition, BBox3D, Transform, CRS};
use crate::remote::{download_from_url, extract_extension_from_url, is_remote_url, url_filename};
use serde_json::Value;
use std::path::{Path, PathBuf};

/// Input source for CityJSON data
///
/// Can be either a local file path or a remote URL
#[derive(Debug, Clone)]
pub enum InputSource {
    /// Local file path
    Local(PathBuf),
    /// Remote URL (http://, https://, s3://, az://, gs://, etc.)
    Remote(String),
}

impl InputSource {
    /// Parse input string into InputSource
    ///
    /// # Arguments
    /// * `input` - Input string (file path or URL)
    ///
    /// # Returns
    /// InputSource enum variant
    pub fn from_str_input(input: &str) -> Result<Self> {
        if is_remote_url(input) {
            Ok(InputSource::Remote(input.to_string()))
        } else {
            Ok(InputSource::Local(PathBuf::from(input)))
        }
    }
}

/// Get a reader from an InputSource
///
/// # Arguments
/// * `source` - InputSource (local path or URL)
///
/// # Returns
/// Box<dyn CityModelMetadataReader>
///
/// # Errors
/// Returns error if:
/// - URL format is unsupported
/// - File not found
/// - Failed to read remote content
pub async fn get_reader_from_source(
    source: &InputSource,
) -> Result<Box<dyn CityModelMetadataReader>> {
    match source {
        InputSource::Local(path) => get_reader(path),
        InputSource::Remote(url) => {
            // Validate extension before downloading to avoid wasting bandwidth
            let extension = extract_extension_from_url(url)?;
            match extension.as_str() {
                "json" | "jsonl" | "cjseq" => {}
                _ => {
                    return Err(CityJsonStacError::InvalidCityJson(format!(
                        "Unsupported remote file extension: {extension}. Supported: .json, .jsonl, .cjseq",
                    )));
                }
            }

            log::info!("Downloading remote file: {}", url);

            let bytes = download_from_url(url).await?;
            let content = String::from_utf8(bytes.to_vec()).map_err(|e| {
                CityJsonStacError::Other(format!("Remote file is not valid UTF-8: {e}"))
            })?;

            let filename = url_filename(url);
            let virtual_path = PathBuf::from(&filename);

            log::debug!(
                "Remote file downloaded: {} bytes, extension: {}",
                content.len(),
                extension
            );

            match extension.as_str() {
                "json" => Ok(Box::new(CityJSONReader::from_content(
                    &content,
                    virtual_path,
                )?)),
                "jsonl" | "cjseq" => Ok(Box::new(CityJSONSeqReader::from_content(
                    &content,
                    virtual_path,
                )?)),
                _ => unreachable!("extension already validated above"),
            }
        }
    }
}

/// Trait for extracting metadata from CityJSON-format files
///
/// Implemented by format-specific readers (CityJSON, CityJSONSeq, FlatCityBuf, etc.)
pub trait CityModelMetadataReader: Send + Sync {
    /// Get the 3D bounding box of the city model
    fn bbox(&self) -> Result<BBox3D>;

    /// Get the coordinate reference system
    fn crs(&self) -> Result<CRS>;

    /// Get the levels of detail present in the model
    fn lods(&self) -> Result<Vec<String>>;

    /// Get the types of city objects present
    fn city_object_types(&self) -> Result<Vec<String>>;

    /// Get the total count of city objects
    fn city_object_count(&self) -> Result<usize>;

    /// Get attribute definitions
    fn attributes(&self) -> Result<Vec<AttributeDefinition>>;

    /// Get the encoding format name
    fn encoding(&self) -> &'static str;

    /// Get the CityJSON version
    fn version(&self) -> Result<String>;

    /// Get the file path
    fn file_path(&self) -> &Path;

    /// Get the coordinate transform if present
    fn transform(&self) -> Result<Option<Transform>>;

    /// Get the metadata object if present
    fn metadata(&self) -> Result<Option<Value>>;

    /// Get the extensions used
    fn extensions(&self) -> Result<Vec<String>>;

    /// Check if semantic surfaces are present
    fn semantic_surfaces(&self) -> Result<bool>;

    /// Check if textures are present
    fn textures(&self) -> Result<bool>;

    /// Check if materials are present
    fn materials(&self) -> Result<bool>;
}

/// Factory function to get the appropriate reader based on file extension
pub fn get_reader(path: &Path) -> Result<Box<dyn CityModelMetadataReader>> {
    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .ok_or_else(|| CityJsonStacError::InvalidCityJson("No file extension".to_string()))?;

    match extension {
        "json" => Ok(Box::new(CityJSONReader::new(path)?)),
        "jsonl" => Ok(Box::new(CityJSONSeqReader::new(path)?)),
        "fcb" => Ok(Box::new(FlatCityBufReader::new(path)?)),
        _ => Err(CityJsonStacError::InvalidCityJson(format!(
            "Unsupported file extension: {extension}",
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_input_source_local() {
        let source = InputSource::from_str_input("tests/data/delft.city.json").unwrap();
        assert!(matches!(source, InputSource::Local(_)));
    }

    #[test]
    fn test_input_source_remote_https() {
        let source = InputSource::from_str_input("https://example.com/data/city.json").unwrap();
        assert!(matches!(source, InputSource::Remote(_)));
    }

    #[test]
    fn test_input_source_remote_s3() {
        let source = InputSource::from_str_input("s3://bucket/path/city.json").unwrap();
        assert!(matches!(source, InputSource::Remote(_)));
    }

    #[test]
    fn test_input_source_remote_azure() {
        let source = InputSource::from_str_input("az://container/city.json").unwrap();
        assert!(matches!(source, InputSource::Remote(_)));
    }

    #[test]
    fn test_input_source_remote_gcs() {
        let source = InputSource::from_str_input("gs://bucket/city.json").unwrap();
        assert!(matches!(source, InputSource::Remote(_)));
    }

    #[tokio::test]
    async fn test_get_reader_from_source_unsupported_remote_extension() {
        let source = InputSource::Remote("https://example.com/data/file.txt".to_string());
        let result = get_reader_from_source(&source).await;
        assert!(result.is_err());
        match result {
            Err(CityJsonStacError::InvalidCityJson(msg)) => {
                assert!(msg.contains("Unsupported remote file extension"));
            }
            _ => panic!("Expected InvalidCityJson error for unsupported extension"),
        }
    }
}
