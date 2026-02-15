//! Remote file access module using object_store
//!
//! Provides unified access to files from multiple storage backends:
//! - Local filesystem
//! - HTTP/HTTPS URLs
//! - Amazon S3 (s3://)
//! - Azure Blob Storage (az://, azure://)
//! - Google Cloud Storage (gs://)

use crate::error::{CityJsonStacError, Result};
use object_store::DynObjectStore;
use std::sync::Arc;
use url::Url;

/// Create an object store from a URL string
///
/// This function creates appropriate ObjectStore implementations based on URL scheme:
/// - `http://` or `https://` → HTTP store
/// - `s3://` → Amazon S3
/// - `az://` or `azure://` → Azure Blob Storage
/// - `gs://` → Google Cloud Storage
/// - file:// or local path → Local filesystem
///
/// # Arguments
/// * `url` - URL string to parse
///
/// # Returns
/// Tuple of (ObjectStore instance, path within store)
///
/// # Errors
/// Returns error if URL scheme is unsupported or credentials are missing
pub async fn create_store_from_url(
    url: &str,
    options: Option<Vec<(&str, &str)>>,
) -> Result<Arc<DynObjectStore>> {
    // Check for HTTP/HTTPS URLs first
    let url = Url::parse(url).map_err(CityJsonStacError::UrlError)?;
    let (store, _path) =
        object_store::parse_url_opts(&url, options.unwrap_or_default()).map_err(|e| {
            CityJsonStacError::StorageError(format!("Failed to create object store: {e}"))
        })?;
    Ok(Arc::from(store))
}

/// Download content from a remote URL as bytes
///
/// Uses `object_store` to support multiple backends:
/// - HTTP/HTTPS URLs
/// - Amazon S3 (`s3://`)
/// - Azure Blob Storage (`az://`, `azure://`)
/// - Google Cloud Storage (`gs://`)
///
/// # Arguments
/// * `url` - Remote URL string
///
/// # Returns
/// Downloaded file content as `bytes::Bytes`
///
/// # Errors
/// Returns error if URL parsing fails, store creation fails, or download fails
pub async fn download_from_url(url: &str) -> Result<bytes::Bytes> {
    let parsed_url = Url::parse(url).map_err(CityJsonStacError::UrlError)?;
    let (store, path) = object_store::parse_url_opts(&parsed_url, Vec::<(String, String)>::new())
        .map_err(|e| {
        CityJsonStacError::StorageError(format!("Failed to create object store: {e}"))
    })?;

    let result = store.get(&path).await?;
    let bytes = result.bytes().await?;
    Ok(bytes)
}

/// Extract file extension from URL or path
///
/// # Arguments
/// * `url` - URL string to extract extension from
///
/// # Returns
/// File extension without dot, or error if no extension found
pub fn extract_extension_from_url(url: &str) -> Result<String> {
    let filename = url
        .split('/')
        .next_back()
        .and_then(|s| s.split('?').next()) // Remove query string
        .ok_or_else(|| {
            CityJsonStacError::Other(format!("No file extension found in URL: {url}"))
        })?;

    // Check if filename contains a dot (has extension)
    if filename.contains('.') {
        filename
            .rsplit('.')
            .next()
            .filter(|ext| !ext.is_empty())
            .ok_or_else(|| {
                CityJsonStacError::Other(format!("No file extension found in URL: {url}"))
            })
            .map(|s| s.to_string())
    } else {
        Err(CityJsonStacError::Other(format!(
            "No file extension found in URL: {url}",
        )))
    }
}

/// Check if a string is a remote URL (not a local file path)
///
/// # Arguments
/// * `input` - String to check
///
/// # Returns
/// true if string appears to be a URL
pub fn is_remote_url(input: &str) -> bool {
    input.starts_with("http://")
        || input.starts_with("https://")
        || input.starts_with("s3://")
        || input.starts_with("az://")
        || input.starts_with("azure://")
        || input.starts_with("gs://")
}

/// Get filename from a URL for display purposes
///
/// # Arguments
/// * `url` - URL string
///
/// # Returns
/// Filename extracted from URL path
pub fn url_filename(url: &str) -> String {
    url.split('/')
        .next_back()
        .and_then(|s| s.split('?').next()) // Remove query string
        .filter(|s| !s.is_empty())
        .unwrap_or("remote.file")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_extension_from_url() {
        assert_eq!(
            extract_extension_from_url("https://example.com/file.json").unwrap(),
            "json"
        );
        assert_eq!(
            extract_extension_from_url("https://example.com/file.city.json").unwrap(),
            "json"
        );
        assert_eq!(
            extract_extension_from_url("https://example.com/file.jsonl?query=1").unwrap(),
            "jsonl"
        );
        assert_eq!(
            extract_extension_from_url("https://example.com/data.cjseq").unwrap(),
            "cjseq"
        );
        assert_eq!(
            extract_extension_from_url("s3://bucket/path/to/file.fcb").unwrap(),
            "fcb"
        );
        assert!(extract_extension_from_url("https://example.com/file").is_err());
        assert!(extract_extension_from_url("https://example.com/").is_err());
    }

    #[test]
    fn test_is_remote_url() {
        assert!(is_remote_url("https://example.com/file.json"));
        assert!(is_remote_url("http://example.com/file.json"));
        assert!(is_remote_url("s3://bucket/path/to/file.json"));
        assert!(is_remote_url("az://container/path/to/file.json"));
        assert!(is_remote_url("azure://container/path/to/file.json"));
        assert!(is_remote_url("gs://bucket/path/to/file.json"));
        assert!(!is_remote_url("file.json"));
        assert!(!is_remote_url("/path/to/file.json"));
        assert!(!is_remote_url("./relative/path.json"));
    }

    #[test]
    fn test_url_filename() {
        assert_eq!(
            url_filename("https://example.com/data/city.json"),
            "city.json"
        );
        assert_eq!(
            url_filename("https://example.com/data/building.city.json?v=2"),
            "building.city.json"
        );
        assert_eq!(url_filename("https://example.com/"), "remote.file");
        assert_eq!(
            url_filename("http://test.example.org/path/to/file.json"),
            "file.json"
        );
        assert_eq!(
            url_filename("s3://my-bucket/path/to/data.cjseq"),
            "data.cjseq"
        );
    }
}
