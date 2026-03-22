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
pub mod gzip;
pub mod zip;

pub use citygml::{CityGMLReader, CityGMLVersion};
pub use cityjson::CityJSONReader;
pub use cjseq::CityJSONSeqReader;
pub use fcb::FlatCityBufReader;
pub use gzip::GzipReader;
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
                "json" | "jsonl" | "cjseq" | "gml" | "xml" | "zip" | "gz" => {}
                _ => {
                    return Err(CityJsonStacError::InvalidCityJson(format!(
                        "Unsupported remote file extension: {extension}. Supported: .json, .jsonl, .cjseq, .gml, .xml, .zip, .gz",
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
                    let temp_path = temp_file.into_temp_path();
                    let real_path = temp_path.to_path_buf();
                    let reader =
                        CityGMLReader::from_temp_file(&virtual_path, &real_path, temp_path)?;
                    Ok(Box::new(reader))
                }
                "zip" => {
                    log::info!("Downloading remote ZIP file: {}", url);
                    let bytes = download_from_url(url).await?;
                    let mut temp_file = tempfile::Builder::new().suffix(".zip").tempfile()?;
                    use std::io::Write;
                    temp_file.write_all(&bytes)?;
                    let temp_path = temp_file.into_temp_path();
                    let real_path = temp_path.to_path_buf();
                    let reader = ZipReader::from_temp_file(&virtual_path, &real_path, temp_path)?;
                    Ok(Box::new(reader))
                }
                "gz" => {
                    log::info!("Downloading remote GZIP file: {}", url);
                    let bytes = download_from_url(url).await?;
                    let mut temp_file = tempfile::Builder::new().suffix(".gz").tempfile()?;
                    use std::io::Write;
                    temp_file.write_all(&bytes)?;
                    let temp_path = temp_file.into_temp_path();
                    let real_path = temp_path.to_path_buf();
                    let reader = GzipReader::from_temp_file(&virtual_path, &real_path, temp_path)?;
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
        .map(|e| e.to_lowercase())
        .ok_or_else(|| CityJsonStacError::InvalidCityJson("No file extension".to_string()))?;

    match extension.as_str() {
        "gz" => Ok(Box::new(GzipReader::new(path)?)),
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

/// Parse a CityJSON string, normalizing non-conforming fields.
///
/// Some older CityJSON files have fields with incorrect types:
/// - `"version"` as integer (e.g. `1`) instead of string (`"1.0"`)
/// - `"lod"` in geometry objects as integer (e.g. `1`) instead of string (`"1"`)
///
/// This function normalizes such fields before passing to the strict `cjseq` parser.
pub(crate) fn parse_cityjson(content: &str) -> std::result::Result<::cjseq::CityJSON, String> {
    // First try direct parsing (fast path for conforming files)
    if let Ok(cj) = ::cjseq::CityJSON::from_str(content) {
        return Ok(cj);
    }

    // If that fails, try to fix known issues
    let mut value: Value =
        serde_json::from_str(content).map_err(|e| format!("Failed to parse JSON: {e}"))?;

    if let Some(obj) = value.as_object_mut() {
        // Normalize version: integer -> string (e.g. 1 -> "1.0", 2 -> "2.0")
        if let Some(version) = obj.get_mut("version") {
            if let Some(n) = version.as_i64() {
                *version = Value::String(format!("{n}.0"));
            } else if let Some(n) = version.as_f64() {
                *version = Value::String(format!("{n}"));
            }
        }

        // Normalize lod fields in geometry objects: integer -> string
        if let Some(city_objects) = obj.get_mut("CityObjects") {
            normalize_lod_fields(city_objects);
        }

        // Normalize referenceSystem: URN -> URL
        // CityJSON 1.0 allowed "urn:ogc:def:crs:EPSG::3414" but cjseq requires
        // "https://www.opengis.net/def/crs/EPSG/0/3414"
        if let Some(metadata) = obj.get_mut("metadata") {
            if let Some(rs) = metadata.get_mut("referenceSystem") {
                if let Some(urn) = rs.as_str() {
                    if let Some(url) = normalize_crs_urn_to_url(urn) {
                        *rs = Value::String(url);
                    }
                }
            }
        }

        // Normalize float vertices to i64, adjusting the transform accordingly.
        // Some non-conforming CityJSON files store vertices as floats (e.g. geographic
        // coordinates in degrees) rather than the required integers. We pick a scale
        // factor that preserves the precision present in the data, convert the floats
        // to integers, and write back an updated "transform" block.
        if let Some(vertices) = obj.get("vertices") {
            if vertices_have_floats(vertices) {
                normalize_float_vertices(obj);
            }
        }
    }

    let fixed_content =
        serde_json::to_string(&value).map_err(|e| format!("Failed to serialize JSON: {e}"))?;
    ::cjseq::CityJSON::from_str(&fixed_content)
        .map_err(|e| format!("Failed to parse CityJSON after normalization: {e}"))
}

/// Recursively normalize `"lod"` fields from integer/float to string
/// within CityObjects geometry arrays.
fn normalize_lod_fields(city_objects: &mut Value) {
    if let Some(objects) = city_objects.as_object_mut() {
        for co in objects.values_mut() {
            if let Some(geometry) = co.get_mut("geometry") {
                if let Some(geom_array) = geometry.as_array_mut() {
                    for geom in geom_array {
                        if let Some(lod) = geom.get_mut("lod") {
                            if let Some(n) = lod.as_i64() {
                                *lod = Value::String(n.to_string());
                            } else if let Some(n) = lod.as_f64() {
                                *lod = Value::String(format!("{n}"));
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Check whether any vertex coordinate in the vertices array is a float (f64)
/// rather than an integer. CityJSON requires `vertices` to be `Vec<Vec<i64>>`.
fn vertices_have_floats(vertices: &Value) -> bool {
    if let Some(arr) = vertices.as_array() {
        for vertex in arr {
            if let Some(coords) = vertex.as_array() {
                for coord in coords {
                    // serde_json represents a JSON number as f64 if it has a
                    // decimal point. as_i64() returns None for such numbers.
                    if coord.is_f64() && coord.as_i64().is_none() {
                        return true;
                    }
                }
            }
        }
    }
    false
}

/// Normalize float vertices to i64 by deriving a scale factor from the
/// maximum number of decimal digits found across all vertex coordinates.
///
/// The transform is adjusted so that real-world positions are preserved:
///   real = vertex_int * scale + translate
///
/// If the file already has a transform, its translate is kept and the scale
/// is updated to match the new integer encoding. If no transform is present,
/// translate is set to `[0.0, 0.0, 0.0]`.
fn normalize_float_vertices(obj: &mut serde_json::Map<String, Value>) {
    let vertices = match obj.get("vertices").and_then(|v| v.as_array()) {
        Some(v) => v.clone(),
        None => return,
    };

    // Determine the maximum number of decimal places across all float coordinates
    // so we can choose a scale that does not lose precision.
    let mut max_decimals: u32 = 0;
    for vertex in &vertices {
        if let Some(coords) = vertex.as_array() {
            for coord in coords {
                if let Some(f) = coord.as_f64() {
                    // Count decimal digits by formatting the float and splitting on '.'.
                    let s = format!("{f}");
                    if let Some(dot_pos) = s.find('.') {
                        let decimals = (s.len() - dot_pos - 1) as u32;
                        if decimals > max_decimals {
                            max_decimals = decimals;
                        }
                    }
                }
            }
        }
    }

    // Scale factor: 10^(-max_decimals)
    let scale = 10f64.powi(-(max_decimals as i32));

    // Retrieve existing translate, or default to [0, 0, 0].
    let translate = obj
        .get("transform")
        .and_then(|t| t.get("translate"))
        .and_then(|t| t.as_array())
        .and_then(|arr| {
            if arr.len() == 3 {
                let x = arr[0].as_f64()?;
                let y = arr[1].as_f64()?;
                let z = arr[2].as_f64()?;
                Some([x, y, z])
            } else {
                None
            }
        })
        .unwrap_or([0.0, 0.0, 0.0]);

    // Convert each float vertex to i64 using the chosen scale.
    // vertex_real = vertex_int * scale + translate
    // => vertex_int = round((vertex_real - translate) / scale)
    let multiplier = 10f64.powi(max_decimals as i32); // 1 / scale
    let new_vertices: Vec<Value> = vertices
        .iter()
        .map(|vertex| {
            let coords: Vec<Value> = vertex
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .enumerate()
                        .map(|(i, coord)| {
                            let f = coord.as_f64().unwrap_or(0.0);
                            let tr = if i < 3 { translate[i] } else { 0.0 };
                            let int_val = ((f - tr) * multiplier).round() as i64;
                            Value::Number(int_val.into())
                        })
                        .collect()
                })
                .unwrap_or_default();
            Value::Array(coords)
        })
        .collect();

    obj.insert("vertices".to_string(), Value::Array(new_vertices));

    // Write the adjusted transform block.
    obj.insert(
        "transform".to_string(),
        serde_json::json!({
            "scale": [scale, scale, scale],
            "translate": translate
        }),
    );
}

/// Convert CRS URN to OGC URL format.
///
/// Handles patterns like:
/// - `urn:ogc:def:crs:EPSG::3414` → `https://www.opengis.net/def/crs/EPSG/0/3414`
/// - `urn:ogc:def:crs:EPSG:0:3414` → `https://www.opengis.net/def/crs/EPSG/0/3414`
///
/// Returns `None` if the input is not a recognized URN pattern.
fn normalize_crs_urn_to_url(urn: &str) -> Option<String> {
    let stripped = urn.strip_prefix("urn:ogc:def:crs:")?;
    // Format: AUTHORITY:VERSION:CODE (e.g., "EPSG::3414" or "EPSG:0:3414")
    let parts: Vec<&str> = stripped.splitn(3, ':').collect();
    if parts.len() == 3 {
        let authority = parts[0];
        let version = if parts[1].is_empty() { "0" } else { parts[1] };
        let code = parts[2];
        if !code.is_empty() {
            return Some(format!(
                "https://www.opengis.net/def/crs/{authority}/{version}/{code}"
            ));
        }
    }
    None
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

    #[test]
    fn test_get_reader_gzip_file() {
        use std::io::Write;

        // Create a GZIP file with CityJSON content
        let temp_gz = tempfile::Builder::new()
            .suffix(".json.gz")
            .tempfile()
            .unwrap();

        let cityjson = r#"{
            "type": "CityJSON",
            "version": "1.1",
            "CityObjects": {},
            "vertices": []
        }"#;

        let mut encoder =
            flate2::write::GzEncoder::new(temp_gz.as_file(), flate2::Compression::default());
        encoder.write_all(cityjson.as_bytes()).unwrap();
        encoder.finish().unwrap();

        let reader = get_reader(temp_gz.path());
        assert!(reader.is_ok());
        assert_eq!(reader.unwrap().encoding(), "CityJSON");
    }

    /// Regression test: CityJSON files with float vertices (e.g. geographic
    /// coordinates stored directly in degrees) must be normalised to i64.
    /// This mirrors the Indiana-*.json error:
    ///   "invalid type: floating point `-85.44525`, expected i64"
    #[test]
    fn test_parse_cityjson_float_vertices_normalized() {
        let cityjson = r#"{
            "type": "CityJSON",
            "version": "2.0",
            "transform": {"scale": [1.0, 1.0, 1.0], "translate": [0, 0, 0]},
            "CityObjects": {},
            "vertices": [[-85.44525, 39.76838, 210.5], [-85.44600, 39.76900, 211.0]]
        }"#;

        let result = parse_cityjson(cityjson);
        assert!(
            result.is_ok(),
            "Expected float vertices to be normalized, got: {:?}",
            result.err()
        );

        let cj = result.unwrap();
        // Vertices must now be integers (i64).
        assert_eq!(cj.vertices.len(), 2);
        for v in &cj.vertices {
            assert_eq!(v.len(), 3, "Each vertex must have 3 coordinates");
        }

        // The chosen scale must reflect at least 5 decimal places (max seen: 85.44525 → 5 dec.).
        // We just verify the transform exists (scale ≠ 1.0) and vertices are non-trivial.
        // The exact values depend on implementation rounding, so we only sanity-check.
        let scale = &cj.transform.scale;
        assert!(scale[0] <= 0.00001, "Scale should be ≤ 1e-5, got {scale:?}");
    }

    #[test]
    fn test_normalize_crs_urn_to_url() {
        assert_eq!(
            normalize_crs_urn_to_url("urn:ogc:def:crs:EPSG::3414"),
            Some("https://www.opengis.net/def/crs/EPSG/0/3414".to_string())
        );
        assert_eq!(
            normalize_crs_urn_to_url("urn:ogc:def:crs:EPSG:0:7415"),
            Some("https://www.opengis.net/def/crs/EPSG/0/7415".to_string())
        );
        // Already URL format - not a URN
        assert_eq!(
            normalize_crs_urn_to_url("https://www.opengis.net/def/crs/EPSG/0/3414"),
            None
        );
    }

    /// Regression test: CityJSON 1.0 files with URN-style referenceSystem
    /// (e.g. "urn:ogc:def:crs:EPSG::3414") must be normalized to URL format.
    #[test]
    fn test_parse_cityjson_urn_reference_system_normalized() {
        let cityjson = r#"{
            "type": "CityJSON",
            "version": "1.0",
            "transform": {"scale": [0.01, 0.01, 0.01], "translate": [0, 0, 0]},
            "metadata": {
                "referenceSystem": "urn:ogc:def:crs:EPSG::3414",
                "geographicalExtent": [0, 0, 0, 100, 100, 50]
            },
            "CityObjects": {},
            "vertices": []
        }"#;

        let result = parse_cityjson(cityjson);
        assert!(
            result.is_ok(),
            "Expected URN referenceSystem to be normalized, got: {:?}",
            result.err()
        );
    }
}
