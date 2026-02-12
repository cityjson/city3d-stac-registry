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
use crate::remote::is_remote_url;
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
        InputSource::Remote(_) => {
            // TODO: Implement remote readers
            // For now, we need to download the remote file and use it
            // Remote reader implementation is planned but not yet available
            Err(CityJsonStacError::Other(
                "Remote readers are not yet implemented. Please use local files.".to_string(),
            ))
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
