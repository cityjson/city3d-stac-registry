//! Error types for the cityjson-stac library

use std::io;
use thiserror::Error;

/// Result type alias for cityjson-stac operations
pub type Result<T> = std::result::Result<T, CityJsonStacError>;

/// Errors that can occur when processing CityJSON files and generating STAC metadata
#[derive(Error, Debug)]
pub enum CityJsonStacError {
    /// Unsupported file format
    #[error("Unsupported file format: {0}\nSupported formats: .json (CityJSON), .jsonl (CityJSONSeq), .gml (CityGML), .fcb (FlatCityBuf)")]
    UnsupportedFormat(String),

    /// IO error (file not found, permission denied, etc.)
    #[error("IO error: {0}")]
    IoError(#[from] io::Error),

    /// JSON parsing error
    #[error("JSON parsing error: {0}")]
    JsonError(#[from] serde_json::Error),

    /// Failed to extract metadata from file
    #[error("Failed to extract metadata: {0}\nThe file may be corrupted or invalid")]
    MetadataError(String),

    /// Invalid CityJSON structure
    #[error("Invalid CityJSON structure: {0}")]
    InvalidCityJson(String),

    /// Missing required field
    #[error("Missing required field: {0}")]
    MissingField(String),

    /// STAC generation error
    #[error("STAC generation error: {0}")]
    StacError(String),

    /// URL parsing error
    #[error("Invalid URL: {0}")]
    UrlError(#[from] url::ParseError),

    /// GeoJSON error
    #[error("GeoJSON error: {0}")]
    GeoJsonError(String),

    /// No supported files found in directory
    #[error("No supported files found in directory")]
    NoFilesFound,

    /// Object storage error (unified error for HTTP, S3, Azure, GCS, local)
    #[error("Object storage error: {0}")]
    ObjectStoreError(#[from] object_store::Error),

    /// Path parsing error for object storage
    #[error("Path error: {0}")]
    PathError(#[from] object_store::path::Error),

    /// Generic network/storage error (kept for backwards compatibility)
    #[error("Storage/network error: {0}")]
    StorageError(String),

    /// Parquet write error
    #[error("Parquet write error: {0}")]
    ParquetError(#[from] parquet::errors::ParquetError),

    /// Arrow error
    #[error("Arrow error: {0}")]
    ArrowError(#[from] arrow::error::ArrowError),

    /// Generic error with custom message
    #[error("{0}")]
    Other(String),
}

impl From<String> for CityJsonStacError {
    fn from(s: String) -> Self {
        CityJsonStacError::Other(s)
    }
}

impl From<&str> for CityJsonStacError {
    fn from(s: &str) -> Self {
        CityJsonStacError::Other(s.to_string())
    }
}

impl From<zip::result::ZipError> for CityJsonStacError {
    fn from(e: zip::result::ZipError) -> Self {
        CityJsonStacError::InvalidCityJson(format!("ZIP archive error: {e}"))
    }
}
