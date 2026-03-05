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

/// Convert S3 HTTPS URLs to s3:// format for object_store
///
/// Handles URLs like:
/// - https://s3.dualstack.us-east-1.amazonaws.com/bucket/key
/// - https://bucket.s3.amazonaws.com/key
/// - https://bucket.s3.region.amazonaws.com/key
fn convert_s3_https_url(url: &str) -> Option<String> {
    // Match S3 dualstack URLs: https://s3.dualstack.{region}.amazonaws.com/{bucket}/{key}
    if let Some(rest) = url.strip_prefix("https://s3.dualstack.") {
        if let Some(region_and_rest) = rest.split_once(".amazonaws.com/") {
            let (region, path) = region_and_rest;
            if let Some((bucket, key)) = path.split_once('/') {
                return Some(format!("s3://{}/{}?region={}", bucket, key, region));
            }
        }
    }

    // Match virtual-hosted style with region: https://{bucket}.s3.{region}.amazonaws.com/{key}
    // This must be checked before the simpler .s3.amazonaws.com pattern
    if let Some(rest) = url.strip_prefix("https://") {
        if let Some((bucket_with_s3, region_and_key)) = rest.split_once(".s3.") {
            if bucket_with_s3.ends_with(".amazonaws.com") {
                // This is the path-style, handled below
            } else if let Some((region, key)) = region_and_key.split_once(".amazonaws.com/") {
                // https://{bucket}.s3.{region}.amazonaws.com/{key}
                return Some(format!("s3://{}/{}?region={}", bucket_with_s3, key, region));
            }
        }
    }

    // Match virtual-hosted style without region: https://{bucket}.s3.amazonaws.com/{key}
    if let Some((bucket, rest)) = url
        .strip_prefix("https://")
        .and_then(|s| s.split_once(".s3.amazonaws.com/"))
    {
        return Some(format!("s3://{}/{}", bucket, rest));
    }

    // Match path-style: https://s3.{region}.amazonaws.com/{bucket}/{key}
    if let Some(rest) = url.strip_prefix("https://s3.") {
        if let Some((region, path)) = rest.split_once(".amazonaws.com/") {
            if let Some((bucket, key)) = path.split_once('/') {
                return Some(format!("s3://{}/{}?region={}", bucket, key, region));
            }
        }
    }

    None
}

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
    // Try to convert S3 HTTPS URLs to s3:// format
    let converted_url = convert_s3_https_url(url);
    let url_to_use = converted_url.as_deref().unwrap_or(url);

    let parsed_url = Url::parse(url_to_use).map_err(CityJsonStacError::UrlError)?;

    // If the URL was converted from HTTPS to s3://, the bucket is being accessed
    // publicly via HTTPS, so we skip AWS credential signing to avoid timeouts
    // when no credentials are available (e.g., EC2 IMDS requests on local machines).
    let options: Vec<(String, String)> = if converted_url.is_some() {
        vec![("aws_skip_signature".to_string(), "true".to_string())]
    } else {
        Vec::new()
    };

    let (store, path) = object_store::parse_url_opts(&parsed_url, options).map_err(|e| {
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
    fn test_convert_s3_dualstack_url() {
        let url = "https://s3.dualstack.us-east-1.amazonaws.com/mybucket/path/to/file.json";
        let converted = convert_s3_https_url(url).unwrap();
        assert_eq!(
            converted,
            "s3://mybucket/path/to/file.json?region=us-east-1"
        );
    }

    #[test]
    fn test_convert_s3_virtual_hosted_url() {
        let url = "https://mybucket.s3.amazonaws.com/path/to/file.json";
        let converted = convert_s3_https_url(url).unwrap();
        assert_eq!(converted, "s3://mybucket/path/to/file.json");
    }

    #[test]
    fn test_convert_s3_virtual_hosted_with_region() {
        let url = "https://mybucket.s3.eu-west-1.amazonaws.com/path/to/file.json";
        let converted = convert_s3_https_url(url).unwrap();
        assert_eq!(
            converted,
            "s3://mybucket/path/to/file.json?region=eu-west-1"
        );
    }

    #[test]
    fn test_convert_non_s3_url() {
        let url = "https://example.com/file.json";
        assert!(convert_s3_https_url(url).is_none());
    }

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
