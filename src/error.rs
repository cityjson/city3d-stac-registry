//! Error types for the cityjson-stac library

use std::io;
use thiserror::Error;

/// Result type alias for cityjson-stac operations
pub type Result<T> = std::result::Result<T, CityJsonStacError>;

/// Errors that can occur when processing CityJSON files and generating STAC metadata
#[derive(Error, Debug)]
pub enum CityJsonStacError {
    /// Unsupported file format
    #[error("Unsupported file format: {0}\nSupported formats: .json (CityJSON), .jsonl (CityJSONSeq), .fcb (FlatCityBuf)")]
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
