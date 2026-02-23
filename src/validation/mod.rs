//! Validation logic for dry-run mode

pub mod result;

use crate::config::CollectionConfigFile;
use crate::error::{CityJsonStacError, Result};
use result::ValidationResult;
use std::path::PathBuf;

/// Validate collection configuration without generating output
pub async fn validate_collection_config(
    config_path: &Option<PathBuf>,
    inputs: &[PathBuf],
    base_url: &Option<String>,
) -> Result<ValidationResult> {
    let mut result = ValidationResult::new();

    // 1. Validate config file syntax if provided
    if let Some(path) = config_path {
        let spinner = console::style("→").blue();
        println!("  {} Checking config file: {}", spinner, path.display());

        match CollectionConfigFile::from_file(path) {
            Ok(_config) => {
                result.config_valid = true;
                println!("  ✓ Config file syntax: valid");
            }
            Err(e) => {
                result.config_valid = false;
                result.config_error = Some(e.to_string());
                println!("  ✗ Config file syntax: {}", e);
            }
        }
    }

    // 2. Validate input paths exist
    if !inputs.is_empty() {
        let mut found = 0;
        let mut missing = Vec::new();

        for path in inputs {
            if path.exists() {
                found += 1;
            } else {
                missing.push(path.clone());
            }
        }

        result.paths_found = found;
        result.paths_total = inputs.len();
        result.missing_paths = missing.clone();

        if missing.is_empty() {
            println!("  ✓ Input paths: {}/{} found", found, inputs.len());
        } else {
            println!("  ⚠ Input paths: {}/{} found", found, inputs.len());
            for path in &missing {
                println!("    ✗ {}", path.display());
            }
        }
    }

    // 3. Validate base URL if provided
    if let Some(url) = base_url {
        println!("  → Checking base URL: {}", url);
        match validate_url_head(url).await {
            Ok(status) => {
                result.base_url_valid = true;
                println!("  ✓ Base URL: accessible ({})", status);
            }
            Err(e) => {
                result.base_url_valid = false;
                result.base_url_error = Some(e.to_string());
                println!("  ✗ Base URL: {}", e);
            }
        }
    }

    Ok(result)
}

/// Validate URL with HEAD request (lightweight, doesn't download body)
async fn validate_url_head(url: &str) -> Result<String> {
    use object_store::path::Path as ObjectStorePath;
    use object_store::ObjectStore;

    // Parse the URL to determine the store type
    let parsed_url = url::Url::parse(url)
        .map_err(|e| CityJsonStacError::Other(format!("Invalid URL: {}", e)))?;

    let store: Box<dyn ObjectStore> = match parsed_url.scheme() {
        "http" | "https" => {
            use object_store::http::HttpBuilder;
            Box::new(
                HttpBuilder::new()
                    .with_url(parsed_url.to_string())
                    .build()
                    .map_err(|e| {
                        CityJsonStacError::Other(format!("Failed to create HTTP store: {}", e))
                    })?,
            )
        }
        "s3" => {
            use object_store::aws::AmazonS3Builder;
            Box::new(
                AmazonS3Builder::from_env()
                    .with_url(parsed_url.to_string())
                    .build()
                    .map_err(|e| {
                        CityJsonStacError::Other(format!("Failed to create S3 store: {}", e))
                    })?,
            )
        }
        "gs" | "gcs" => {
            use object_store::gcp::GoogleCloudStorageBuilder;
            Box::new(
                GoogleCloudStorageBuilder::from_env()
                    .with_url(parsed_url.to_string())
                    .build()
                    .map_err(|e| {
                        CityJsonStacError::Other(format!("Failed to create GCS store: {}", e))
                    })?,
            )
        }
        "azure" | "az" => {
            use object_store::azure::MicrosoftAzureBuilder;
            Box::new(
                MicrosoftAzureBuilder::from_env()
                    .with_url(parsed_url.to_string())
                    .build()
                    .map_err(|e| {
                        CityJsonStacError::Other(format!("Failed to create Azure store: {}", e))
                    })?,
            )
        }
        scheme => {
            return Err(CityJsonStacError::Other(format!(
                "Unsupported URL scheme: {}",
                scheme
            )))
        }
    };

    // Extract path from URL (remove scheme and authority)
    let path = ObjectStorePath::from_url_path(parsed_url.path())
        .map_err(|e| CityJsonStacError::Other(format!("Invalid path: {}", e)))?;

    // Check if the location exists
    match store.head(&path).await {
        Ok(_) => Ok("200 OK".to_string()),
        Err(e) => Err(CityJsonStacError::Other(format!("URL check failed: {}", e))),
    }
}

/// Validate item input (file path or URL)
pub async fn validate_item_input(input: &str) -> Result<ValidationResult> {
    let mut result = ValidationResult::new();

    // Check if it's a remote URL
    if input.starts_with("http://") || input.starts_with("https://") {
        println!("  → Checking remote URL: {}", input);
        match validate_url_head(input).await {
            Ok(status) => {
                result.base_url_valid = true;
                println!("  ✓ URL: accessible ({})", status);
            }
            Err(e) => {
                result.base_url_valid = false;
                result.base_url_error = Some(e.to_string());
                println!("  ✗ URL: {}", e);
            }
        }
    } else {
        // Local file
        let path = PathBuf::from(input);
        println!("  → Checking local file: {}", input);

        if path.exists() {
            result.paths_found = 1;
            result.paths_total = 1;
            println!("  ✓ File: exists");
        } else {
            result.paths_total = 1;
            result.missing_paths.push(path);
            println!("  ✗ File: not found");
        }
    }

    Ok(result)
}
