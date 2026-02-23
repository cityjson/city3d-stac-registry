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
        result.missing_paths = missing;

        if result.missing_paths.is_empty() {
            println!("  ✓ Input paths: {}/{} found", found, inputs.len());
        } else {
            println!("  ⚠ Input paths: {}/{} found", found, inputs.len());
            for path in &result.missing_paths {
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
    use reqwest::Client;
    use std::time::Duration;

    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| CityJsonStacError::Other(format!("Failed to create HTTP client: {}", e)))?;

    let response = client
        .head(url)
        .send()
        .await
        .map_err(|e| CityJsonStacError::Other(format!("HTTP request failed: {}", e)))?;

    let status = response.status();

    if status.is_success() {
        Ok(status.to_string())
    } else {
        Err(CityJsonStacError::Other(format!("HTTP {}", status)))
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
