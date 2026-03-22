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
/// For HTTP/HTTPS URLs, uses `reqwest` directly for maximum compatibility
/// with diverse web servers (some servers omit standard headers like
/// `Content-Length` during transparent decompression, which `object_store`
/// requires but `reqwest` does not).
///
/// For cloud storage URLs (s3://, gs://, az://), uses `object_store` for
/// native protocol support and credential handling.
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

    match parsed_url.scheme() {
        // For cloud storage schemes, use object_store with native protocol support
        "s3" | "gs" | "az" | "azure" => {
            let options: Vec<(String, String)> = if converted_url.is_some() {
                vec![("aws_skip_signature".to_string(), "true".to_string())]
            } else {
                Vec::new()
            };

            let (store, path) =
                object_store::parse_url_opts(&parsed_url, options).map_err(|e| {
                    CityJsonStacError::StorageError(format!("Failed to create object store: {e}"))
                })?;

            let result = store.get(&path).await?;
            let bytes = result.bytes().await?;
            Ok(bytes)
        }
        // For HTTP/HTTPS, use reqwest directly to avoid object_store's strict
        // header requirements (e.g. Content-Length) that some servers don't provide
        "http" | "https" => {
            let response = reqwest::get(url_to_use).await.map_err(|e| {
                CityJsonStacError::StorageError(format!("HTTP request failed: {e}"))
            })?;

            if !response.status().is_success() {
                return Err(CityJsonStacError::StorageError(format!(
                    "HTTP {} for {}",
                    response.status(),
                    url
                )));
            }

            let bytes = response.bytes().await.map_err(|e| {
                CityJsonStacError::StorageError(format!("Failed to read response body: {e}"))
            })?;
            Ok(bytes)
        }
        scheme => Err(CityJsonStacError::StorageError(format!(
            "Unsupported URL scheme: {scheme}"
        ))),
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
    // First try the path component (before query string)
    let last_segment = url.split('/').next_back().unwrap_or("");
    let path_part = last_segment.split('?').next().unwrap_or("");

    if let Some(ext) = extract_ext_from_filename(path_part) {
        let lower = ext.to_lowercase();
        // If the path extension is a known data format, use it directly
        match lower.as_str() {
            "json" | "jsonl" | "cjseq" | "gml" | "xml" | "zip" | "gz" | "fcb" => return Ok(lower),
            _ => {}
        }
    }

    // Fallback: check query parameters for a filename (e.g., ?file=data.gml&id=4 or ?files=data.gml)
    if let Some(query) = url.split('?').nth(1) {
        for param in query.split('&') {
            if let Some(value) = param
                .strip_prefix("file=")
                .or_else(|| param.strip_prefix("files="))
                .or_else(|| param.strip_prefix("f="))
            {
                // URL-decode the value and extract extension
                let decoded = value.replace("%2F", "/").replace("%2E", ".");
                let filename = decoded.split('/').next_back().unwrap_or(value);
                if let Some(ext) = extract_ext_from_filename(filename) {
                    return Ok(ext.to_lowercase());
                }
            }
        }
    }

    // If path had any extension (even non-data like .php), return it
    if let Some(ext) = extract_ext_from_filename(path_part) {
        return Ok(ext.to_lowercase());
    }

    Err(CityJsonStacError::Other(format!(
        "No file extension found in URL: {url}",
    )))
}

/// Extract extension from a filename string (without dot)
fn extract_ext_from_filename(filename: &str) -> Option<&str> {
    if filename.contains('.') {
        filename.rsplit('.').next().filter(|ext| !ext.is_empty())
    } else {
        None
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
    // First check query parameters for a filename (e.g., ?file=data.gml, ?files=data.gml, or ?f=data.zip)
    if let Some(query) = url.split('?').nth(1) {
        for param in query.split('&') {
            if let Some(value) = param
                .strip_prefix("file=")
                .or_else(|| param.strip_prefix("files="))
                .or_else(|| param.strip_prefix("f="))
            {
                let decoded = value.replace("%2F", "/").replace("%2E", ".");
                let filename = decoded.split('/').next_back().unwrap_or(value);
                if filename.contains('.') {
                    return filename.to_string();
                }
            }
        }
    }
    // Fall back to the path component
    url.split('/')
        .next_back()
        .and_then(|s| s.split('?').next())
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
        // Case-insensitive
        assert_eq!(
            extract_extension_from_url("https://example.com/file.GML").unwrap(),
            "gml"
        );
        assert_eq!(
            extract_extension_from_url("https://example.com/file.ZIP").unwrap(),
            "zip"
        );
        // Query parameter: ?file=data.gml
        assert_eq!(
            extract_extension_from_url("https://example.com/download?file=data.gml&id=4").unwrap(),
            "gml"
        );
        // Query parameter: ?f=data.zip (Estonia-style)
        assert_eq!(
            extract_extension_from_url(
                "https://example.com/index.php?f=hooned_lod2-citygml.zip&page_id=837"
            )
            .unwrap(),
            "zip"
        );
        // PHP with file= query param should prefer the file param extension
        assert_eq!(
            extract_extension_from_url("https://example.com/massen.php?file=LoD2_data.xml&id=4")
                .unwrap(),
            "xml"
        );
        // Query parameter: ?files=data.gml (Nextcloud-style)
        assert_eq!(
            extract_extension_from_url(
                "https://example.com/s/opendata/download?path=%2F3d&files=city_model.gml"
            )
            .unwrap(),
            "gml"
        );
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
        // Nextcloud-style download URL with files= query param
        assert_eq!(
            url_filename("https://example.com/s/opendata/download?path=%2F3d&files=city_model.gml"),
            "city_model.gml"
        );
    }
}
