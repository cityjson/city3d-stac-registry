//! Reader implementations for different CityJSON formats
//!
//! This module provides a unified approach to reading CityJSON files
//! from both local filesystem and remote storage (HTTP, S3, Azure, GCS)
//! using the object_store crate.
//!
pub mod citygml;
pub mod cityjson;
pub mod cjseq;
pub mod fcb;
pub mod zip;

pub use citygml::{CityGMLReader, CityGMLVersion};
pub use cityjson::CityJSONReader;
pub use cjseq::CityJSONSeqReader;
pub use fcb::FlatCityBufReader;
pub use zip::ZipReader;

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
                "json" | "jsonl" | "cjseq" | "gml" | "xml" | "zip" => {}
                _ => {
                    return Err(CityJsonStacError::InvalidCityJson(format!(
                        "Unsupported remote file extension: {extension}. Supported: .json, .jsonl, .cjseq, .gml, .xml, .zip",
                    )));
                }
            }

            let filename = url_filename(url);
            let virtual_path = PathBuf::from(&filename);

            match extension.as_str() {
                "json" => {
                    // CityJSON: not streamable, download entire file then parse
                    log::info!("Downloading remote CityJSON file: {}", url);
                    let bytes = download_from_url(url).await?;
                    let content = String::from_utf8(bytes.to_vec()).map_err(|e| {
                        CityJsonStacError::Other(format!("Remote file is not valid UTF-8: {e}"))
                    })?;
                    log::debug!("Downloaded {} bytes for {}", content.len(), filename);
                    Ok(Box::new(CityJSONReader::from_content(
                        &content,
                        virtual_path,
                    )?))
                }
                "jsonl" | "cjseq" => {
                    // CityJSONSeq: streamable, process line-by-line as data arrives
                    log::info!("Streaming remote CityJSONSeq file: {}", url);
                    Ok(Box::new(
                        CityJSONSeqReader::from_url_stream(url, virtual_path).await?,
                    ))
                }
                "gml" | "xml" => {
                    log::info!("Downloading remote CityGML file: {}", url);
                    let bytes = download_from_url(url).await?;
                    // write temporary file to handle it local
                    let mut temp_file = tempfile::Builder::new()
                        .suffix(&format!(".{}", extension))
                        .tempfile()?;
                    use std::io::Write;
                    temp_file.write_all(&bytes)?;
                    let path = temp_file.path().to_path_buf();
                    let reader =
                        CityGMLReader::new(&path)?.with_temp_path(temp_file.into_temp_path());
                    Ok(Box::new(reader))
                }
                "zip" => {
                    log::info!("Downloading remote ZIP file: {}", url);
                    let bytes = download_from_url(url).await?;
                    let mut temp_file = tempfile::Builder::new().suffix(".zip").tempfile()?;
                    use std::io::Write;
                    temp_file.write_all(&bytes)?;
                    let path = temp_file.path().to_path_buf();
                    let reader = ZipReader::from_temp_file(path, temp_file.into_temp_path())?;
                    Ok(Box::new(reader))
                }
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

    /// Whether this format supports streaming I/O
    ///
    /// Streaming formats (e.g., CityJSONSeq) can be processed line-by-line
    /// without buffering the entire file into memory. This is especially
    /// beneficial for remote files where data can be processed as it arrives.
    fn streamable(&self) -> bool {
        false
    }
}

/// Factory function to get the appropriate reader based on file extension
pub fn get_reader(path: &Path) -> Result<Box<dyn CityModelMetadataReader>> {
    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .ok_or_else(|| CityJsonStacError::InvalidCityJson("No file extension".to_string()))?;

    match extension {
        "zip" => Ok(Box::new(ZipReader::new(path)?)),
        "json" => Ok(Box::new(CityJSONReader::new(path)?)),
        "jsonl" => Ok(Box::new(CityJSONSeqReader::new(path)?)),
        "fcb" => Ok(Box::new(FlatCityBufReader::new(path)?)),
        "gml" | "xml" => {
            // Check if it's a valid CityGML file
            if is_citygml(path)? {
                Ok(Box::new(CityGMLReader::new(path)?))
            } else {
                Err(CityJsonStacError::UnsupportedFormat(format!(
                    "File is not a valid CityGML file: {extension}"
                )))
            }
        }
        _ => Err(CityJsonStacError::InvalidCityJson(format!(
            "Unsupported file extension: {extension}",
        ))),
    }
}

/// Quick check if file is CityGML by looking for namespace in first few KB
fn is_citygml(path: &Path) -> Result<bool> {
    use std::io::{BufRead, BufReader};
    let file = std::fs::File::open(path)?;
    let reader = BufReader::new(file);

    for line in reader.lines().take(20) {
        let line = line?;
        if line.contains("citygml") || line.contains("www.opengis.net/gml") {
            return Ok(true);
        }
    }
    Ok(false)
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

    #[test]
    fn test_cityjson_reader_not_streamable() {
        use std::io::Write;

        let mut temp_file = tempfile::Builder::new().suffix(".json").tempfile().unwrap();
        let cityjson = r#"{
            "type": "CityJSON",
            "version": "2.0",
            "transform": {"scale": [1.0, 1.0, 1.0], "translate": [0, 0, 0]},
            "metadata": {"geographicalExtent": [0, 0, 0, 1, 1, 1]},
            "CityObjects": {},
            "vertices": []
        }"#;
        writeln!(temp_file, "{}", cityjson).unwrap();

        let reader = get_reader(temp_file.path()).unwrap();
        assert!(!reader.streamable());
    }

    #[test]
    fn test_cjseq_reader_streamable() {
        use std::io::Write;

        let mut temp_file = tempfile::Builder::new()
            .suffix(".jsonl")
            .tempfile()
            .unwrap();
        let header = r#"{"type":"CityJSON","version":"2.0","transform":{"scale":[1.0,1.0,1.0],"translate":[0,0,0]},"CityObjects":{},"vertices":[],"metadata":{"geographicalExtent":[0,0,0,1,1,1]}}"#;
        let feature = r#"{"type":"CityJSONFeature","id":"b1","CityObjects":{"b1":{"type":"Building","geometry":[]}},"vertices":[]}"#;
        writeln!(temp_file, "{}", header).unwrap();
        writeln!(temp_file, "{}", feature).unwrap();
        temp_file.flush().unwrap();

        let reader = get_reader(temp_file.path()).unwrap();
        assert!(reader.streamable());
    }

    #[test]
    fn test_get_reader_zip_file() {
        use ::zip::write::SimpleFileOptions;
        use ::zip::CompressionMethod;
        use ::zip::ZipWriter;
        use std::io::Write;

        // Create a ZIP file with CityJSON content
        let temp_zip = tempfile::Builder::new().suffix(".zip").tempfile().unwrap();
        let mut zip = ZipWriter::new(temp_zip.as_file());

        let cityjson = r#"{
            "type": "CityJSON",
            "version": "1.1",
            "CityObjects": {},
            "vertices": []
        }"#;

        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
        zip.start_file("test.json", options).unwrap();
        zip.write_all(cityjson.as_bytes()).unwrap();
        zip.finish().unwrap();

        let reader = get_reader(temp_zip.path());
        assert!(reader.is_ok());
        assert_eq!(reader.unwrap().encoding(), "CityJSON");
    }
}
