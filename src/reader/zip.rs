//! ZIP archive reader for CityJSON/CityGML files
//!
//! Extracts ZIP archives and aggregates metadata from all supported files inside.

use crate::error::{CityJsonStacError, Result};
use crate::metadata::AttributeDefinition;
use crate::metadata::BBox3D;
use crate::metadata::CRS;
use crate::reader::{get_reader, CityModelMetadataReader};
use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::sync::RwLock;
use tempfile::{TempDir, TempPath};

/// Reader for ZIP archives containing CityJSON/CityGML files
pub struct ZipReader {
    file_path: PathBuf,
    temp_dir: TempDir,
    /// Keep temp file alive for remote ZIPs (downloaded files)
    _temp_file: Option<TempPath>,
    inner_readers: Vec<Box<dyn CityModelMetadataReader>>,
    metadata: RwLock<Option<ZipMetadata>>,
}

/// Aggregated metadata from all files in the ZIP
#[derive(Debug)]
struct ZipMetadata {
    bbox: Option<BBox3D>,
    city_object_count: usize,
    city_object_types: BTreeSet<String>,
    lods: BTreeSet<String>,
    attributes: Vec<AttributeDefinition>,
    primary_encoding: &'static str,
    version: String,
    crs: Option<CRS>,
    has_textures: bool,
    has_materials: bool,
    has_semantic_surfaces: bool,
}

impl ZipReader {
    /// Create a new ZIP reader from a local file
    pub fn new(file_path: &Path) -> Result<Self> {
        if !file_path.exists() {
            return Err(CityJsonStacError::IoError(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("File not found: {}", file_path.display()),
            )));
        }

        // Create temporary directory for extraction
        let temp_dir = TempDir::new()?;

        // Extract ZIP to temp directory
        Self::extract_zip(file_path, temp_dir.path())?;

        let mut reader = Self {
            file_path: file_path.to_path_buf(),
            temp_dir,
            _temp_file: None,
            inner_readers: Vec::new(),
            metadata: RwLock::new(None),
        };

        // Discover and create inner readers
        reader.inner_readers = reader.discover_inner_readers()?;

        if reader.inner_readers.is_empty() {
            return Err(CityJsonStacError::InvalidCityJson(
                "No CityJSON/CityGML files found in ZIP".to_string(),
            ));
        }

        Ok(reader)
    }

    /// Create a ZIP reader from a temporary file (for remote downloads)
    ///
    /// This constructor takes ownership of a TempPath to keep the downloaded
    /// ZIP file alive for the lifetime of the reader.
    pub fn from_temp_file(file_path: PathBuf, temp_path: TempPath) -> Result<Self> {
        // Create temporary directory for extraction
        let temp_dir = TempDir::new()?;

        // Extract ZIP to temp directory
        Self::extract_zip(&file_path, temp_dir.path())?;

        let mut reader = Self {
            file_path,
            temp_dir,
            _temp_file: Some(temp_path),
            inner_readers: Vec::new(),
            metadata: RwLock::new(None),
        };

        // Discover and create inner readers
        reader.inner_readers = reader.discover_inner_readers()?;

        if reader.inner_readers.is_empty() {
            return Err(CityJsonStacError::InvalidCityJson(
                "No CityJSON/CityGML files found in ZIP".to_string(),
            ));
        }

        Ok(reader)
    }

    /// Extract ZIP file to directory
    fn extract_zip(zip_path: &Path, dest_dir: &Path) -> Result<()> {
        let file = std::fs::File::open(zip_path)?;
        let mut archive = zip::ZipArchive::new(file)?;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let outpath = dest_dir.join(file.name());

            // Validate path doesn't escape dest_dir (ZIP Slip prevention)
            if !outpath.starts_with(dest_dir) {
                return Err(CityJsonStacError::InvalidCityJson(format!(
                    "ZIP file contains unsafe path: {}",
                    file.name()
                )));
            }

            if file.name().ends_with('/') {
                std::fs::create_dir_all(&outpath)?;
            } else {
                if let Some(parent) = outpath.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                let mut outfile = std::fs::File::create(&outpath)?;
                std::io::copy(&mut file, &mut outfile)?;
            }
        }

        Ok(())
    }

    /// Discover all supported files in extracted directory
    fn discover_inner_readers(&self) -> Result<Vec<Box<dyn CityModelMetadataReader>>> {
        let mut readers = Vec::new();

        // Walk the extracted directory
        fn walk_dir(dir: &Path, readers: &mut Vec<Box<dyn CityModelMetadataReader>>) -> Result<()> {
            for entry in std::fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();

                if path.is_dir() {
                    walk_dir(&path, readers)?;
                } else {
                    // Try to create a reader for this file
                    if let Ok(reader) = get_reader(&path) {
                        log::debug!("Found supported file in ZIP: {:?}", path);
                        readers.push(reader);
                    }
                }
            }
            Ok(())
        }

        walk_dir(self.temp_dir.path(), &mut readers)?;
        Ok(readers)
    }

    /// Aggregate metadata from all inner readers
    fn aggregate_metadata(&self) -> Result<ZipMetadata> {
        let mut city_object_count = 0;
        let mut city_object_types = BTreeSet::new();
        let mut lods = BTreeSet::new();
        let mut attributes_map: HashMap<String, AttributeDefinition> = HashMap::new();
        let mut has_textures = false;
        let mut has_materials = false;
        let mut has_semantic_surfaces = false;

        // For bbox, we need to merge extents
        let mut min_x = f64::MAX;
        let mut min_y = f64::MAX;
        let mut min_z = f64::MAX;
        let mut max_x = f64::MIN;
        let mut max_y = f64::MIN;
        let mut max_z = f64::MIN;
        let mut has_bbox = false;

        let primary_encoding = self
            .inner_readers
            .first()
            .map(|r| r.encoding())
            .unwrap_or("CityJSON");

        let mut version = String::new();
        let mut crs = None;

        for reader in &self.inner_readers {
            // Count city objects
            if let Ok(count) = reader.city_object_count() {
                city_object_count += count;
            }

            // Collect city object types
            if let Ok(types) = reader.city_object_types() {
                city_object_types.extend(types);
            }

            // Collect LODs
            if let Ok(reader_lods) = reader.lods() {
                lods.extend(reader_lods);
            }

            // Collect attributes (using HashMap for deduplication by name)
            if let Ok(reader_attrs) = reader.attributes() {
                for attr in reader_attrs {
                    attributes_map.insert(attr.name.clone(), attr);
                }
            }

            // Check for textures/materials/semantic surfaces
            if let Ok(true) = reader.textures() {
                has_textures = true;
            }
            if let Ok(true) = reader.materials() {
                has_materials = true;
            }
            if let Ok(true) = reader.semantic_surfaces() {
                has_semantic_surfaces = true;
            }

            // Merge bbox
            if let Ok(bbox) = reader.bbox() {
                has_bbox = true;
                min_x = min_x.min(bbox.xmin);
                min_y = min_y.min(bbox.ymin);
                min_z = min_z.min(bbox.zmin);
                max_x = max_x.max(bbox.xmax);
                max_y = max_y.max(bbox.ymax);
                max_z = max_z.max(bbox.zmax);
            }

            // Get version and CRS from first reader
            if version.is_empty() {
                if let Ok(v) = reader.version() {
                    version = v;
                }
            }
            if crs.is_none() {
                if let Ok(c) = reader.crs() {
                    crs = Some(c);
                }
            }
        }

        let bbox = if has_bbox {
            Some(BBox3D::new(min_x, min_y, min_z, max_x, max_y, max_z))
        } else {
            None
        };

        let attributes: Vec<_> = attributes_map.into_values().collect();

        Ok(ZipMetadata {
            bbox,
            city_object_count,
            city_object_types,
            lods,
            attributes,
            primary_encoding,
            version,
            crs,
            has_textures,
            has_materials,
            has_semantic_surfaces,
        })
    }

    /// Lazy load metadata
    fn ensure_loaded(&self) -> Result<()> {
        {
            let metadata = self
                .metadata
                .read()
                .map_err(|_| CityJsonStacError::Other("Failed to acquire read lock".to_string()))?;
            if metadata.is_some() {
                return Ok(());
            }
        }

        let mut metadata = self
            .metadata
            .write()
            .map_err(|_| CityJsonStacError::Other("Failed to acquire write lock".to_string()))?;

        if metadata.is_none() {
            *metadata = Some(self.aggregate_metadata()?);
        }

        Ok(())
    }
}

impl CityModelMetadataReader for ZipReader {
    fn bbox(&self) -> Result<BBox3D> {
        self.ensure_loaded()?;
        let metadata = self
            .metadata
            .read()
            .map_err(|_| CityJsonStacError::Other("Failed to acquire read lock".to_string()))?;
        metadata
            .as_ref()
            .and_then(|m| m.bbox.clone())
            .ok_or_else(|| CityJsonStacError::MetadataError("BBox not found".to_string()))
    }

    fn crs(&self) -> Result<CRS> {
        self.ensure_loaded()?;
        let metadata = self
            .metadata
            .read()
            .map_err(|_| CityJsonStacError::Other("Failed to acquire read lock".to_string()))?;
        Ok(metadata.as_ref().unwrap().crs.clone().unwrap_or_default())
    }

    fn lods(&self) -> Result<Vec<String>> {
        self.ensure_loaded()?;
        let metadata = self
            .metadata
            .read()
            .map_err(|_| CityJsonStacError::Other("Failed to acquire read lock".to_string()))?;
        Ok(metadata.as_ref().unwrap().lods.iter().cloned().collect())
    }

    fn city_object_types(&self) -> Result<Vec<String>> {
        self.ensure_loaded()?;
        let metadata = self
            .metadata
            .read()
            .map_err(|_| CityJsonStacError::Other("Failed to acquire read lock".to_string()))?;
        Ok(metadata
            .as_ref()
            .unwrap()
            .city_object_types
            .iter()
            .cloned()
            .collect())
    }

    fn city_object_count(&self) -> Result<usize> {
        self.ensure_loaded()?;
        let metadata = self
            .metadata
            .read()
            .map_err(|_| CityJsonStacError::Other("Failed to acquire read lock".to_string()))?;
        Ok(metadata.as_ref().unwrap().city_object_count)
    }

    fn attributes(&self) -> Result<Vec<AttributeDefinition>> {
        self.ensure_loaded()?;
        let metadata = self
            .metadata
            .read()
            .map_err(|_| CityJsonStacError::Other("Failed to acquire read lock".to_string()))?;
        Ok(metadata.as_ref().unwrap().attributes.clone())
    }

    fn encoding(&self) -> &'static str {
        // Return the internal format from first file found
        if let Ok(metadata) = self.metadata.read() {
            if let Some(ref m) = *metadata {
                return m.primary_encoding;
            }
        }
        "CityJSON" // Fallback
    }

    fn version(&self) -> Result<String> {
        self.ensure_loaded()?;
        let metadata = self
            .metadata
            .read()
            .map_err(|_| CityJsonStacError::Other("Failed to acquire read lock".to_string()))?;
        Ok(metadata.as_ref().unwrap().version.clone())
    }

    fn file_path(&self) -> &Path {
        &self.file_path
    }

    fn transform(&self) -> Result<Option<crate::metadata::Transform>> {
        Ok(None) // ZIP wrapper doesn't use vertex compression
    }

    fn metadata(&self) -> Result<Option<serde_json::Value>> {
        Ok(None)
    }

    fn extensions(&self) -> Result<Vec<String>> {
        Ok(Vec::new())
    }

    fn semantic_surfaces(&self) -> Result<bool> {
        self.ensure_loaded()?;
        let metadata = self
            .metadata
            .read()
            .map_err(|_| CityJsonStacError::Other("Failed to acquire read lock".to_string()))?;
        Ok(metadata.as_ref().unwrap().has_semantic_surfaces)
    }

    fn textures(&self) -> Result<bool> {
        self.ensure_loaded()?;
        let metadata = self
            .metadata
            .read()
            .map_err(|_| CityJsonStacError::Other("Failed to acquire read lock".to_string()))?;
        Ok(metadata.as_ref().unwrap().has_textures)
    }

    fn materials(&self) -> Result<bool> {
        self.ensure_loaded()?;
        let metadata = self
            .metadata
            .read()
            .map_err(|_| CityJsonStacError::Other("Failed to acquire read lock".to_string()))?;
        Ok(metadata.as_ref().unwrap().has_materials)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    /// Helper function to create a test ZIP file with CityJSON content
    fn create_test_zip_with_cityjson() -> NamedTempFile {
        let temp_zip = NamedTempFile::new().unwrap();
        let mut zip = zip::ZipWriter::new(temp_zip.as_file());

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

        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);

        zip.start_file("data.city.json", options).unwrap();
        zip.write_all(cityjson.as_bytes()).unwrap();
        zip.finish().unwrap();

        temp_zip
    }

    #[test]
    fn test_zip_reader_aggregates_metadata() {
        let temp_zip = create_test_zip_with_cityjson();
        let reader = ZipReader::new(temp_zip.path()).unwrap();

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
    fn test_zip_reader_empty_zip() {
        let temp_zip = NamedTempFile::new().unwrap();
        let zip = zip::ZipWriter::new(temp_zip.as_file());
        zip.finish().unwrap();

        let result = ZipReader::new(temp_zip.path());
        assert!(result.is_err());
        match result {
            Err(CityJsonStacError::InvalidCityJson(msg)) => {
                assert!(msg.contains("No CityJSON/CityGML files found"));
            }
            _ => panic!("Expected InvalidCityJson error"),
        }
    }

    #[test]
    fn test_zip_reader_not_streamable() {
        // Create a minimal valid ZIP file
        let mut temp_zip = NamedTempFile::new().unwrap();
        let zip = zip::ZipWriter::new(temp_zip.as_file_mut());
        zip.finish().unwrap();

        // Note: This will fail with "No CityJSON/CityGML files found"
        // because the ZIP is empty. That's expected - test verifies
        // the struct compiles and method exists.
        let result = ZipReader::new(temp_zip.path());
        assert!(result.is_err());

        // Verify it's the expected error
        match result {
            Err(CityJsonStacError::InvalidCityJson(msg)) => {
                assert!(msg.contains("No CityJSON/CityGML files found"));
            }
            _ => panic!("Expected InvalidCityJson error"),
        }
    }
}
