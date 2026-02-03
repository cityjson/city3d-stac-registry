//! Reader implementations for different CityJSON formats

mod cityjson;
mod cjseq;
// mod fcb;    // FlatCityBuf - to be implemented

pub use cityjson::CityJSONReader;
pub use cjseq::CityJSONSeqReader;

use crate::error::{CityJsonStacError, Result};
use crate::metadata::{AttributeDefinition, BBox3D, Transform, CRS};
use std::path::Path;

/// Trait for extracting metadata from CityJSON-format files
///
/// Implemented by format-specific readers (CityJSON, CityJSONSeq, FlatCityBuf, etc.)
pub trait CityModelMetadataReader: Send + Sync {
    /// Extract 3D bounding box [xmin, ymin, zmin, xmax, ymax, zmax]
    ///
    /// Returns the spatial extent of all geometry in the file.
    /// Values should be in the native CRS of the dataset.
    fn bbox(&self) -> Result<BBox3D>;

    /// Get coordinate reference system information
    ///
    /// Returns EPSG code and WKT2 representation if available
    fn crs(&self) -> Result<CRS>;

    /// Get list of available Levels of Detail
    ///
    /// Returns strings like ["0", "1", "2", "2.2"]
    fn lods(&self) -> Result<Vec<String>>;

    /// Get list of CityObject types present
    ///
    /// Returns types like ["Building", "BuildingPart", "Road"]
    fn city_object_types(&self) -> Result<Vec<String>>;

    /// Count total number of city objects
    fn city_object_count(&self) -> Result<usize>;

    /// Extract attribute schema definitions
    ///
    /// Returns schema describing semantic attributes attached to objects
    fn attributes(&self) -> Result<Vec<AttributeDefinition>>;

    /// Get encoding format name
    ///
    /// Returns one of: "CityJSON", "CityJSONSeq", "FlatCityBuf", "CityParquet"
    fn encoding(&self) -> &'static str;

    /// Get CityJSON version
    ///
    /// Returns version string like "2.0" or "1.1"
    fn version(&self) -> Result<String>;

    /// Get file path being read
    fn file_path(&self) -> &Path;

    /// Get coordinate transform parameters if present
    ///
    /// Returns scale and translate arrays for vertex compression
    fn transform(&self) -> Result<Option<Transform>>;

    /// Extract additional metadata from file
    ///
    /// Returns free-form metadata object from CityJSON
    fn metadata(&self) -> Result<Option<serde_json::Value>>;
}

/// Factory function to create appropriate reader for a file
///
/// # Arguments
/// * `file_path` - Path to the file to read
///
/// # Returns
/// Boxed trait object implementing CityModelMetadataReader
///
/// # Errors
/// Returns error if file format is unsupported or file cannot be opened
pub fn get_reader(file_path: &Path) -> Result<Box<dyn CityModelMetadataReader>> {
    let extension = file_path
        .extension()
        .and_then(|e| e.to_str())
        .ok_or_else(|| {
            CityJsonStacError::UnsupportedFormat("No file extension found".to_string())
        })?;

    match extension.to_lowercase().as_str() {
        "json" => {
            // Need to peek inside to distinguish CityJSON from regular JSON
            if is_cityjson(file_path)? {
                Ok(Box::new(CityJSONReader::new(file_path)?))
            } else {
                Err(CityJsonStacError::InvalidCityJson(
                    "File is not a CityJSON file (missing 'type': 'CityJSON')".to_string(),
                ))
            }
        }
        "jsonl" | "cjseq" => {
            // CityJSON Text Sequences
            Ok(Box::new(CityJSONSeqReader::new(file_path)?))
        }
        "fcb" => {
            // FlatCityBuf - to be implemented
            Err(CityJsonStacError::UnsupportedFormat(
                "FlatCityBuf (.fcb) support coming soon".to_string(),
            ))
        }
        "parquet" => {
            // Future support
            Err(CityJsonStacError::UnsupportedFormat(
                "CityParquet (.parquet) not yet supported".to_string(),
            ))
        }
        _ => Err(CityJsonStacError::UnsupportedFormat(format!(
            "Unknown extension: {}",
            extension
        ))),
    }
}

/// Helper to check if JSON file is CityJSON format
fn is_cityjson(file_path: &Path) -> Result<bool> {
    use std::fs::File;
    use std::io::{BufRead, BufReader};

    let file = File::open(file_path)?;
    let reader = BufReader::new(file);

    // Read first few lines and look for "type": "CityJSON"
    for line in reader.lines().take(20) {
        let line = line?;
        if line.contains(r#""type""#) && line.contains(r#""CityJSON""#) {
            return Ok(true);
        }
        // Also check for the pattern with spaces
        if line.contains("\"type\"") && line.contains("\"CityJSON\"") {
            return Ok(true);
        }
    }

    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_is_cityjson() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, r#"{{"type": "CityJSON", "version": "2.0"}}"#).unwrap();
        temp_file.flush().unwrap();

        assert!(is_cityjson(temp_file.path()).unwrap());
    }

    #[test]
    fn test_is_not_cityjson() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, r#"{{"name": "test", "value": 123}}"#).unwrap();
        temp_file.flush().unwrap();

        assert!(!is_cityjson(temp_file.path()).unwrap());
    }

    #[test]
    fn test_get_reader_unsupported_extension() {
        let path = Path::new("test.txt");
        let result = get_reader(path);
        assert!(result.is_err());
    }
}
