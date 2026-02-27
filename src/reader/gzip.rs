//! GZIP reader for compressed CityJSON/CityGML files
//!
//! Decompresses .gz files and delegates to the appropriate inner reader
//! based on the uncompressed file extension.

use crate::error::{CityJsonStacError, Result};
use crate::metadata::{AttributeDefinition, BBox3D, Transform, CRS};
use crate::reader::{get_reader, CityModelMetadataReader};
use flate2::read::GzDecoder;
use serde_json::Value;
use std::path::{Path, PathBuf};
use tempfile::{TempDir, TempPath};

/// Reader for GZIP-compressed CityJSON/CityGML files
///
/// This reader decompresses .gz files and delegates to the appropriate
/// inner reader based on the uncompressed file extension.
pub struct GzipReader {
    /// Virtual path for display (e.g., original filename from URL)
    file_path: PathBuf,
    /// Temp directory containing the decompressed file (kept for RAII)
    _temp_dir: TempDir,
    /// Keep temp file alive for remote GZIPs (downloaded files)
    _temp_file: Option<TempPath>,
    /// The inner reader for the decompressed content
    inner_reader: Box<dyn CityModelMetadataReader>,
}

impl GzipReader {
    /// Create a new GZIP reader from a local file
    ///
    /// # Arguments
    /// * `file_path` - Path to the .gz file
    ///
    /// # Returns
    /// A GzipReader that delegates to the appropriate inner reader
    ///
    /// # Errors
    /// Returns error if:
    /// - File not found
    /// - File is not valid GZIP
    /// - Inner file type is not supported
    pub fn new(file_path: &Path) -> Result<Self> {
        if !file_path.exists() {
            return Err(CityJsonStacError::IoError(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("File not found: {}", file_path.display()),
            )));
        }

        // Create temporary directory for decompressed file
        let temp_dir = TempDir::new()?;

        // Determine the inner filename by stripping .gz extension
        let inner_filename = Self::get_inner_filename(file_path);
        let decompressed_path = temp_dir.path().join(&inner_filename);

        // Decompress the file
        Self::decompress_gzip(file_path, &decompressed_path)?;

        // Create inner reader
        let inner_reader = get_reader(&decompressed_path).map_err(|e| {
            CityJsonStacError::InvalidCityJson(format!(
                "Failed to create reader for decompressed file '{}': {}",
                inner_filename, e
            ))
        })?;

        Ok(Self {
            file_path: file_path.to_path_buf(),
            _temp_dir: temp_dir,
            _temp_file: None,
            inner_reader,
        })
    }

    /// Create a GZIP reader from a temporary file (for remote downloads)
    ///
    /// This constructor takes ownership of a TempPath to keep the downloaded
    /// GZIP file alive for the lifetime of the reader.
    ///
    /// # Arguments
    /// * `virtual_path` - The display path (e.g., original filename from URL)
    /// * `real_path` - The actual file path to read from (temp file)
    /// * `temp_path` - The TempPath to keep alive for the lifetime of the reader
    pub fn from_temp_file(
        virtual_path: &Path,
        real_path: &Path,
        temp_path: TempPath,
    ) -> Result<Self> {
        // Create temporary directory for decompressed file
        let temp_dir = TempDir::new()?;

        // Determine the inner filename by stripping .gz extension
        let inner_filename = Self::get_inner_filename(virtual_path);
        let decompressed_path = temp_dir.path().join(&inner_filename);

        // Decompress the file
        Self::decompress_gzip(real_path, &decompressed_path)?;

        // Create inner reader
        let inner_reader = get_reader(&decompressed_path).map_err(|e| {
            CityJsonStacError::InvalidCityJson(format!(
                "Failed to create reader for decompressed file '{}': {}",
                inner_filename, e
            ))
        })?;

        Ok(Self {
            file_path: virtual_path.to_path_buf(),
            _temp_dir: temp_dir,
            _temp_file: Some(temp_path),
            inner_reader,
        })
    }

    /// Get the inner filename by stripping .gz extension
    ///
    /// If the file is named `data.city.json.gz`, returns `data.city.json`.
    /// If the file is just `data.gz` (no double extension), returns `data`.
    fn get_inner_filename(path: &Path) -> String {
        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        // Strip .gz extension
        if filename.to_lowercase().ends_with(".gz") {
            filename[..filename.len() - 3].to_string()
        } else {
            filename.to_string()
        }
    }

    /// Decompress a GZIP file to a target path
    ///
    /// # Arguments
    /// * `gzip_path` - Path to the .gz file
    /// * `dest_path` - Path to write the decompressed content
    fn decompress_gzip(gzip_path: &Path, dest_path: &Path) -> Result<()> {
        let input_file = std::fs::File::open(gzip_path)?;
        let mut decoder = GzDecoder::new(input_file);

        let mut output_file = std::fs::File::create(dest_path)?;
        std::io::copy(&mut decoder, &mut output_file)?;

        log::info!(
            "Decompressed {} to {}",
            gzip_path.display(),
            dest_path.display()
        );

        Ok(())
    }
}

impl CityModelMetadataReader for GzipReader {
    fn bbox(&self) -> Result<BBox3D> {
        self.inner_reader.bbox()
    }

    fn crs(&self) -> Result<CRS> {
        self.inner_reader.crs()
    }

    fn lods(&self) -> Result<Vec<String>> {
        self.inner_reader.lods()
    }

    fn city_object_types(&self) -> Result<Vec<String>> {
        self.inner_reader.city_object_types()
    }

    fn city_object_count(&self) -> Result<usize> {
        self.inner_reader.city_object_count()
    }

    fn attributes(&self) -> Result<Vec<AttributeDefinition>> {
        self.inner_reader.attributes()
    }

    fn encoding(&self) -> &'static str {
        // Return the inner format's encoding
        self.inner_reader.encoding()
    }

    fn version(&self) -> Result<String> {
        self.inner_reader.version()
    }

    fn file_path(&self) -> &Path {
        &self.file_path
    }

    fn transform(&self) -> Result<Option<Transform>> {
        self.inner_reader.transform()
    }

    fn metadata(&self) -> Result<Option<Value>> {
        self.inner_reader.metadata()
    }

    fn extensions(&self) -> Result<Vec<String>> {
        self.inner_reader.extensions()
    }

    fn semantic_surfaces(&self) -> Result<bool> {
        self.inner_reader.semantic_surfaces()
    }

    fn textures(&self) -> Result<bool> {
        self.inner_reader.textures()
    }

    fn materials(&self) -> Result<bool> {
        self.inner_reader.materials()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    /// Helper function to create a test GZIP file with CityJSON content
    fn create_test_gzip_with_cityjson() -> NamedTempFile {
        let temp_gz = NamedTempFile::with_suffix(".city.json.gz").unwrap();

        let cityjson = r#"{
            "type": "CityJSON",
            "version": "1.1",
            "transform": {
                "scale": [0.01, 0.01, 0.01],
                "translate": [100000, 200000, 0]
            },
            "metadata": {
                "geographicalExtent": [1.0, 2.0, 0.0, 10.0, 20.0, 30.0],
                "referenceSystem": "https://www.opengis.net/def/crs/EPSG/0/7415"
            },
            "CityObjects": {
                "building1": {
                    "type": "Building",
                    "geometry": [{
                        "type": "Solid",
                        "lod": "2",
                        "boundaries": [[[[0,0,0]]]]
                    }]
                }
            },
            "vertices": [[0,0,0]]
        }"#;

        // Compress the content
        let mut encoder =
            flate2::write::GzEncoder::new(temp_gz.as_file(), flate2::Compression::default());
        encoder.write_all(cityjson.as_bytes()).unwrap();
        encoder.finish().unwrap();

        temp_gz
    }

    #[test]
    fn test_gzip_reader_delegates_to_inner_reader() {
        let temp_gz = create_test_gzip_with_cityjson();
        let reader = GzipReader::new(temp_gz.path()).unwrap();

        // Should have bbox from inner file
        let bbox = reader.bbox().unwrap();
        assert_eq!(bbox.xmin, 1.0);
        assert_eq!(bbox.xmax, 10.0);

        // Should have city object count
        let count = reader.city_object_count().unwrap();
        assert_eq!(count, 1);

        // Should have city object types
        let types = reader.city_object_types().unwrap();
        assert!(types.contains(&"Building".to_string()));

        // Should have LODs
        let lods = reader.lods().unwrap();
        assert!(lods.contains(&"2".to_string()));

        // Should have version
        let version = reader.version().unwrap();
        assert_eq!(version, "1.1");

        // Should have CRS
        let crs = reader.crs().unwrap();
        assert_eq!(crs.to_stac_epsg(), Some(7415));

        // Should have CityJSON encoding
        assert_eq!(reader.encoding(), "CityJSON");
    }

    #[test]
    fn test_gzip_reader_file_not_found() {
        let result = GzipReader::new(Path::new("/nonexistent/file.json.gz"));
        assert!(result.is_err());
        match result {
            Err(CityJsonStacError::IoError(_)) => {}
            _ => panic!("Expected IoError"),
        }
    }

    #[test]
    fn test_gzip_reader_invalid_gzip() {
        // Create a file that's not actually gzipped
        let mut temp_file = NamedTempFile::with_suffix(".json.gz").unwrap();
        temp_file.write_all(b"not gzipped content").unwrap();

        let result = GzipReader::new(temp_file.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_get_inner_filename() {
        assert_eq!(
            GzipReader::get_inner_filename(Path::new("data.city.json.gz")),
            "data.city.json"
        );
        assert_eq!(
            GzipReader::get_inner_filename(Path::new("data.jsonl.gz")),
            "data.jsonl"
        );
        assert_eq!(
            GzipReader::get_inner_filename(Path::new("data.gml.gz")),
            "data.gml"
        );
        assert_eq!(GzipReader::get_inner_filename(Path::new("data.GZ")), "data");
    }

    #[test]
    fn test_gzip_reader_not_streamable() {
        let temp_gz = create_test_gzip_with_cityjson();
        let reader = GzipReader::new(temp_gz.path()).unwrap();
        // CityJSON is not streamable
        assert!(!reader.streamable());
    }

    #[test]
    fn test_gzip_reader_from_temp_file() {
        // Create a gzipped CityJSON file
        let temp_gz = create_test_gzip_with_cityjson();

        // Read the file content
        let bytes = std::fs::read(temp_gz.path()).unwrap();

        // Create a temp file for the "downloaded" content
        let mut temp_download = NamedTempFile::with_suffix(".city.json.gz").unwrap();
        std::io::Write::write_all(&mut temp_download, &bytes).unwrap();

        let temp_path = temp_download.into_temp_path();
        let real_path = temp_path.to_path_buf();
        let virtual_path = PathBuf::from("remote.city.json.gz");

        let reader = GzipReader::from_temp_file(&virtual_path, &real_path, temp_path).unwrap();

        // Should have metadata from inner file
        let bbox = reader.bbox().unwrap();
        assert_eq!(bbox.xmin, 1.0);

        // File path should be the virtual path
        assert_eq!(reader.file_path(), Path::new("remote.city.json.gz"));
    }

    /// Helper function to create a test GZIP file with CityGML content
    fn create_test_gzip_with_citygml() -> NamedTempFile {
        let temp_gz = NamedTempFile::with_suffix(".gml.gz").unwrap();

        let citygml = r#"<?xml version="1.0" encoding="UTF-8"?>
<core:CityModel xmlns:core="http://www.opengis.net/citygml/2.0"
                xmlns:gml="http://www.opengis.net/gml"
                xmlns:bldg="http://www.opengis.net/citygml/building/2.0">
  <gml:boundedBy>
    <gml:Envelope srsName="urn:ogc:def:crs:EPSG::7415">
      <gml:lowerCorner>1.0 2.0 0.0</gml:lowerCorner>
      <gml:upperCorner>10.0 20.0 30.0</gml:upperCorner>
    </gml:Envelope>
  </gml:boundedBy>
  <core:cityObjectMember>
    <bldg:Building gml:id="building1">
      <bldg:lod2Solid>
        <gml:Solid>
          <gml:exterior>
            <gml:CompositeSurface>
              <gml:surfaceMember/>
            </gml:CompositeSurface>
          </gml:exterior>
        </gml:Solid>
      </bldg:lod2Solid>
    </bldg:Building>
  </core:cityObjectMember>
</core:CityModel>
"#;

        // Compress the content
        let mut encoder =
            flate2::write::GzEncoder::new(temp_gz.as_file(), flate2::Compression::default());
        std::io::Write::write_all(&mut encoder, citygml.as_bytes()).unwrap();
        encoder.finish().unwrap();

        temp_gz
    }

    #[test]
    fn test_gzip_reader_with_citygml() {
        let temp_gz = create_test_gzip_with_citygml();
        let reader = GzipReader::new(temp_gz.path()).unwrap();

        // Should have CityGML encoding
        assert_eq!(reader.encoding(), "CityGML");

        // Should have city object types
        let types = reader.city_object_types().unwrap();
        assert!(types.contains(&"Building".to_string()));

        // Should have LODs
        let lods = reader.lods().unwrap();
        assert!(lods.contains(&"2".to_string()));
    }
}
