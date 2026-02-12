//! Remote file access module using object_store
//!
//! Provides unified access to files from multiple storage backends:
//! - Local filesystem
//! - HTTP/HTTPS URLs
//! - Amazon S3 (s3://)
//! - Azure Blob Storage (az://, azure://)
//! - Google Cloud Storage (gs://)

use crate::error::{CityJsonStacError, Result};
use crate::reader::InputSource;
use bytes::Bytes;
use object_store::{path::Path as ObjectPath, DynObjectStore, ObjectStore};
use std::sync::Arc;

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
pub async fn create_store_from_url(url: &str) -> Result<(Arc<DynObjectStore>, ObjectPath)> {
    // Check for HTTP/HTTPS URLs first
    if url.starts_with("http://") || url.starts_with("https://") {
        let parsed = url::Url::parse(url)
            .map_err(|e| CityJsonStacError::StorageError(format!("Invalid URL: {e}")))?;

        // Extract base URL (scheme + host)
        let base_url = format!(
            "{}://{}",
            parsed.scheme(),
            parsed.host_str().unwrap_or("localhost")
        );

        let store = object_store::http::HttpBuilder::new()
            .with_url(&base_url)
            .build()
            .map_err(|e| {
                CityJsonStacError::StorageError(format!("Failed to create HTTP store: {e}"))
            })?;

        let path = parsed.path().to_string();

        let parsed_path = ObjectPath::from(path.as_str());

        Ok((Arc::new(store), parsed_path))
    } else if url.starts_with("s3://") {
        // S3 URLs are s3://bucket/key
        let store = object_store::aws::AmazonS3Builder::from_env()
            .with_url(url)
            .build()
            .map_err(|e| {
                CityJsonStacError::StorageError(format!("Failed to create S3 store: {e}"))
            })?;

        Ok((
            Arc::new(store),
            ObjectPath::parse(url.strip_prefix("s3://").unwrap_or("")).map_err(|e| {
                CityJsonStacError::StorageError(format!("Failed to parse S3 path: {e}"))
            })?,
        ))
    } else if url.starts_with("az://") || url.starts_with("azure://") {
        // Azure URLs are az://container/key or azure://container/key
        let store = object_store::azure::MicrosoftAzureBuilder::from_env()
            .with_url(url)
            .build()
            .map_err(|e| {
                CityJsonStacError::StorageError(format!("Failed to create Azure store: {e}"))
            })?;

        Ok((
            Arc::new(store),
            ObjectPath::parse(
                url.strip_prefix("az://")
                    .or(Some("azure://"))
                    .unwrap_or("")
                    .strip_prefix("azure://")
                    .unwrap_or(""),
            )
            .map_err(|e| {
                CityJsonStacError::StorageError(format!("Failed to parse Azure path: {e}"))
            })?,
        ))
    } else if url.starts_with("gs://") {
        // GCS URLs are gs://bucket/key
        let store = object_store::gcp::GoogleCloudStorageBuilder::from_env()
            .with_url(url)
            .build()
            .map_err(|e| {
                CityJsonStacError::StorageError(format!("Failed to create GCS store: {e}"))
            })?;

        Ok((
            Arc::new(store),
            ObjectPath::parse(url.strip_prefix("gs://").unwrap_or("")).map_err(|e| {
                CityJsonStacError::StorageError(format!("Failed to parse GCS path: {e}"))
            })?,
        ))
    } else {
        // Not an object_store URL, treat as local file path
        let local = object_store::local::LocalFileSystem::new();
        let path = ObjectPath::from(url);
        Ok((Arc::new(local), path))
    }
}

/// Download content to memory using object_store
///
/// # Arguments
/// * `source` - InputSource (local path or URL)
///
/// # Returns
/// Downloaded bytes
///
/// # Errors
/// Returns error if:
/// - Object store operation fails
/// - File/object not found
/// - Access denied
pub async fn fetch_to_bytes(source: &InputSource) -> Result<Bytes> {
    match source {
        InputSource::Local(path_buf) => {
            // For local files, use tokio fs directly (faster than object_store)
            let bytes = tokio::fs::read(path_buf).await.map_err(|e| {
                CityJsonStacError::StorageError(format!("Failed to read file: {e}"))
            })?;
            Ok(Bytes::from(bytes))
        }
        InputSource::Remote(url) => {
            // Fetch from object store
            let (store, path) = create_store_from_url(url).await?;

            let get_result = store.get(&path).await;

            let bytes = get_result
                .map_err(|e| CityJsonStacError::StorageError(format!("Failed to fetch: {e}")))?
                .bytes()
                .await
                .map_err(|e| {
                    CityJsonStacError::StorageError(format!("Failed to read bytes: {e}"))
                })?;

            Ok(bytes)
        }
    }
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
