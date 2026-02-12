//! CityJSON Text Sequences (CityJSONSeq) format reader
//!
//! Reads `.city.jsonl` or `.cjseq` files which contain JSON Text Sequences
//! as specified in the CityJSON 2.0 specification.
//!
//! The format consists of:
//! - First line: CityJSON header with metadata, transform, and empty CityObjects/vertices
//! - Subsequent lines: CityJSONFeature objects, each with their own vertices

use crate::error::{CityJsonStacError, Result};
use crate::metadata::{AttributeDefinition, AttributeType, BBox3D, Transform, CRS};
use crate::reader::CityModelMetadataReader;
use serde_json::Value;
use std::collections::{BTreeSet, HashMap};
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::RwLock;

/// Data structure to hold parsed CityJSONSeq content
pub(crate) struct CityJSONSeqData {
    /// The header (first line CityJSON object)
    header: Value,
    /// All CityJSONFeature objects (subsequent lines)
    features: Vec<Value>,
}

/// Reader for CityJSON Text Sequences format files (.city.jsonl, .jsonl, .cjseq)
///
/// Uses `RwLock` for interior mutability to enable lazy loading
/// while maintaining thread-safety (`Send + Sync` bounds).
pub struct CityJSONSeqReader {
    file_path: PathBuf,
    /// Cached parsed data (lazy loaded via interior mutability)
    data: RwLock<Option<CityJSONSeqData>>,
}

impl CityJSONSeqReader {
    /// Create a new CityJSONSeq reader
    pub fn new(file_path: &Path) -> Result<Self> {
        if !file_path.exists() {
            return Err(CityJsonStacError::IoError(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("File not found: {}", file_path.display()),
            )));
        }

        Ok(Self {
            file_path: file_path.to_path_buf(),
            data: RwLock::new(None),
        })
    }

    /// Lazy load and cache CityJSONSeq data using interior mutability
    fn ensure_loaded(&self) -> Result<()> {
        // First check if already loaded with a read lock (cheaper)
        {
            let data = self
                .data
                .read()
                .map_err(|_| CityJsonStacError::Other("Failed to acquire read lock".to_string()))?;
            if data.is_some() {
                return Ok(());
            }
        }

        // Not loaded, acquire write lock and load
        let mut data = self
            .data
            .write()
            .map_err(|_| CityJsonStacError::Other("Failed to acquire write lock".to_string()))?;

        // Double-check after acquiring write lock
        if data.is_none() {
            let file = File::open(&self.file_path)?;
            let reader = BufReader::new(file);
            let mut lines = reader.lines();

            // Parse first line as header (CityJSON object)
            let header_line = lines.next().ok_or_else(|| {
                CityJsonStacError::InvalidCityJson("Empty CityJSONSeq file".to_string())
            })??;

            let header: Value = serde_json::from_str(&header_line)?;

            // Validate that first line is a CityJSON header
            let obj_type = header.get("type").and_then(|t| t.as_str());
            if obj_type != Some("CityJSON") {
                return Err(CityJsonStacError::InvalidCityJson(
                    "First line of CityJSONSeq must be a CityJSON object".to_string(),
                ));
            }

            // Parse remaining lines as features
            let mut features = Vec::new();
            for line_result in lines {
                let line = line_result?;
                if line.trim().is_empty() {
                    continue;
                }
                let feature: Value = serde_json::from_str(&line)?;
                features.push(feature);
            }

            *data = Some(CityJSONSeqData { header, features });
        }

        Ok(())
    }

    /// Execute a closure with access to loaded data
    fn with_data<T, F>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&CityJSONSeqData) -> Result<T>,
    {
        self.ensure_loaded()?;
        let data = self
            .data
            .read()
            .map_err(|_| CityJsonStacError::Other("Failed to acquire read lock".to_string()))?;
        let value = data
            .as_ref()
            .expect("data should be loaded after ensure_loaded");
        f(value)
    }
}

/// Helper function to extract f64 from JSON array with proper error handling
fn get_f64_from_array(arr: &[Value], idx: usize) -> Result<f64> {
    arr.get(idx).and_then(|v| v.as_f64()).ok_or_else(|| {
        CityJsonStacError::InvalidCityJson(format!(
            "Expected number at index {}, got: {:?}",
            idx,
            arr.get(idx)
        ))
    })
}

/// Extract transform from CityJSONSeq header
pub(crate) fn extract_transform_from_header(header: &Value) -> Result<Option<Transform>> {
    if let Some(transform) = header.get("transform") {
        let scale = transform
            .get("scale")
            .and_then(|s| s.as_array())
            .and_then(|arr| {
                if arr.len() == 3 {
                    Some([arr[0].as_f64()?, arr[1].as_f64()?, arr[2].as_f64()?])
                } else {
                    None
                }
            });

        let translate = transform
            .get("translate")
            .and_then(|t| t.as_array())
            .and_then(|arr| {
                if arr.len() == 3 {
                    Some([arr[0].as_f64()?, arr[1].as_f64()?, arr[2].as_f64()?])
                } else {
                    None
                }
            });

        if let (Some(scale), Some(translate)) = (scale, translate) {
            return Ok(Some(Transform::new(scale, translate)));
        }
    }

    Ok(None)
}

/// Extract bbox from CityJSONSeq data
/// First tries to header's geographicalExtent, then computes from all features
pub(crate) fn extract_bbox_from_data(data: &CityJSONSeqData) -> Result<BBox3D> {
    // Try to get from header's metadata.geographicalExtent first
    if let Some(metadata) = data.header.get("metadata") {
        if let Some(extent) = metadata.get("geographicalExtent") {
            if let Some(arr) = extent.as_array() {
                if arr.len() == 6 {
                    return Ok(BBox3D::new(
                        get_f64_from_array(arr, 0)?,
                        get_f64_from_array(arr, 1)?,
                        get_f64_from_array(arr, 2)?,
                        get_f64_from_array(arr, 3)?,
                        get_f64_from_array(arr, 4)?,
                        get_f64_from_array(arr, 5)?,
                    ));
                }
            }
        }
    }

    // Fallback: compute bbox from all feature geographicalExtents
    let mut xmin = f64::MAX;
    let mut ymin = f64::MAX;
    let mut zmin = f64::MAX;
    let mut xmax = f64::MIN;
    let mut ymax = f64::MIN;
    let mut zmax = f64::MIN;

    let transform = extract_transform_from_header(&data.header)?;

    for feature in &data.features {
        // Each CityJSONFeature has its own vertices
        if let Some(vertices) = feature.get("vertices") {
            if let Some(vertex_array) = vertices.as_array() {
                for vertex in vertex_array {
                    if let Some(v) = vertex.as_array() {
                        if v.len() >= 3 {
                            let coords = if let Some(ref t) = transform {
                                // Apply transform if present (vertices are compressed integers)
                                let compressed = [
                                    v[0].as_i64().unwrap_or(0) as i32,
                                    v[1].as_i64().unwrap_or(0) as i32,
                                    v[2].as_i64().unwrap_or(0) as i32,
                                ];
                                t.apply(&compressed)
                            } else {
                                // No transform, vertices are raw floats
                                [
                                    v[0].as_f64().unwrap_or(0.0),
                                    v[1].as_f64().unwrap_or(0.0),
                                    v[2].as_f64().unwrap_or(0.0),
                                ]
                            };

                            xmin = xmin.min(coords[0]);
                            ymin = ymin.min(coords[1]);
                            zmin = zmin.min(coords[2]);
                            xmax = xmax.max(coords[0]);
                            ymax = ymax.max(coords[1]);
                            zmax = zmax.max(coords[2]);
                        }
                    }
                }
            }
        }
    }

    Ok(BBox3D::new(xmin, ymin, zmin, xmax, ymax, zmax))
}

/// Extract CRS from CityJSONSeq header
pub(crate) fn extract_crs_from_header(header: &Value) -> Result<CRS> {
    if let Some(metadata) = header.get("metadata") {
        if let Some(ref_system) = metadata.get("referenceSystem") {
            if let Some(url) = ref_system.as_str() {
                if let Some(crs) = CRS::from_cityjson_url(url) {
                    return Ok(crs);
                }
            }
        }
    }

    // Default to WGS84
    Ok(CRS::default())
}

/// Extract LODs from CityJSONSeq features
pub(crate) fn extract_lods_from_features(features: &[Value]) -> Result<Vec<String>> {
    let mut lods = BTreeSet::new();

    for feature in features {
        if let Some(geometry) = feature.get("geometry") {
            if let Some(geom_array) = geometry.as_array() {
                for geom in geom_array {
                    if let Some(lod) = geom.get("lod") {
                        if let Some(lod_num) = lod.as_f64() {
                            lods.insert(lod_num.to_string());
                        } else if let Some(lod_str) = lod.as_str() {
                            lods.insert(lod_str.to_string());
                        }
                    }
                }
            }
        }
    }

    // BTreeSet is already sorted, just collect to Vec
    Ok(lods.into_iter().collect())
}

/// Extract city object types from CityJSONSeq features
pub(crate) fn extract_city_object_types_from_features(features: &[Value]) -> Result<Vec<String>> {
    let mut types = BTreeSet::new();

    for feature in features {
        if let Some(obj_type) = feature.get("type") {
            if let Some(type_str) = obj_type.as_str() {
                types.insert(type_str.to_string());
            }
        }
    }

    // BTreeSet is already sorted, just collect to Vec
    Ok(types.into_iter().collect())
}

/// Extract attribute schema from CityJSONSeq features
pub(crate) fn extract_attributes_from_features(
    features: &[Value],
) -> Result<Vec<AttributeDefinition>> {
    let mut attribute_map: HashMap<String, AttributeType> = HashMap::new();

    for feature in features {
        if let Some(attributes) = feature.get("attributes") {
            if let Some(attrs) = attributes.as_object() {
                for (key, value) in attrs {
                    let attr_type = AttributeType::from_json_value(value);
                    attribute_map.insert(key.clone(), attr_type);
                }
            }
        }
    }

    let mut attributes: Vec<AttributeDefinition> = attribute_map
        .into_iter()
        .map(|(name, attr_type)| AttributeDefinition::new(name, attr_type))
        .collect();

    attributes.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(attributes)
}

/// Extract CityJSON version from header
pub(crate) fn extract_version_from_header(header: &Value) -> Result<String> {
    if let Some(version) = header.get("version") {
        if let Some(v_str) = version.as_str() {
            return Ok(v_str.to_string());
        }
    }

    Ok("1.0".to_string()) // Default version
}

/// Extract CityJSON extensions from header
pub(crate) fn extract_extensions_from_header(header: &Value) -> Result<Vec<String>> {
    let mut extensions = Vec::new();

    if let Some(ext_obj) = header.get("extensions") {
        if let Some(obj) = ext_obj.as_object() {
            // Extensions are stored as {url: name}, we return the URLs
            for url in obj.keys() {
                extensions.push(url.clone());
            }
        }
    }

    // Sort for consistent output
    extensions.sort();
    Ok(extensions)
}

impl CityModelMetadataReader for CityJSONSeqReader {
    fn bbox(&self) -> Result<BBox3D> {
        self.with_data(extract_bbox_from_data)
    }

    fn crs(&self) -> Result<CRS> {
        self.with_data(|data| extract_crs_from_header(&data.header))
    }

    fn lods(&self) -> Result<Vec<String>> {
        self.with_data(|data| extract_lods_from_features(&data.features))
    }

    fn city_object_types(&self) -> Result<Vec<String>> {
        self.with_data(|data| extract_city_object_types_from_features(&data.features))
    }

    fn city_object_count(&self) -> Result<usize> {
        self.with_data(|data| Ok(data.features.len()))
    }

    fn attributes(&self) -> Result<Vec<AttributeDefinition>> {
        self.with_data(|data| extract_attributes_from_features(&data.features))
    }

    fn encoding(&self) -> &'static str {
        "CityJSONSeq"
    }

    fn version(&self) -> Result<String> {
        self.with_data(|data| extract_version_from_header(&data.header))
    }

    fn file_path(&self) -> &Path {
        &self.file_path
    }

    fn transform(&self) -> Result<Option<Transform>> {
        self.with_data(|data| extract_transform_from_header(&data.header))
    }

    fn metadata(&self) -> Result<Option<Value>> {
        self.with_data(|data| Ok(Some(data.header.clone())))
    }

    fn extensions(&self) -> Result<Vec<String>> {
        self.with_data(|data| extract_extensions_from_header(&data.header))
    }

    fn semantic_surfaces(&self) -> Result<bool> {
        self.with_data(|data| {
            // Check if any feature has geometry (CityJSONSeq features can have geometry)
            Ok(data.features.iter().any(|f| f.get("geometry").is_some()))
        })
    }

    fn textures(&self) -> Result<bool> {
        self.with_data(|data| Ok(data.header.get("textures").is_some()))
    }

    fn materials(&self) -> Result<bool> {
        self.with_data(|data| Ok(data.header.get("materials").is_some()))
    }
}

/// Wrapper functions for extracting from Bytes directly
/// These parse CityJSONSeq from bytes and call the main extraction functions
#[allow(dead_code)]
pub(crate) fn extract_bbox_from_bytes(bytes: &bytes::Bytes) -> Result<BBox3D> {
    // Parse CityJSONSeq from bytes - use the from_bytes helper function from local reader
    // We use the CityJSONSeqReader directly for bytes

    let mut temp_file = tempfile::NamedTempFile::new()?;
    // Write bytes to temp file
    std::fs::write(&temp_file, bytes)?;
    temp_file.flush()?;

    // Now use the reader
    let reader = CityJSONSeqReader::new(temp_file.path())?;
    reader.bbox()
}

#[allow(dead_code)]
pub(crate) fn extract_crs_from_bytes(bytes: &bytes::Bytes) -> Result<CRS> {
    let mut temp_file = tempfile::NamedTempFile::new()?;
    std::fs::write(&temp_file, bytes)?;
    temp_file.flush()?;

    let reader = CityJSONSeqReader::new(temp_file.path())?;
    reader.crs()
}

#[allow(dead_code)]
pub(crate) fn extract_lods_from_bytes(bytes: &bytes::Bytes) -> Result<Vec<String>> {
    let mut temp_file = tempfile::NamedTempFile::new()?;
    std::fs::write(&temp_file, bytes)?;
    temp_file.flush()?;

    let reader = CityJSONSeqReader::new(temp_file.path())?;
    reader.lods()
}

#[allow(dead_code)]
pub(crate) fn extract_city_object_types_from_bytes(bytes: &bytes::Bytes) -> Result<Vec<String>> {
    let mut temp_file = tempfile::NamedTempFile::new()?;
    std::fs::write(&temp_file, bytes)?;
    temp_file.flush()?;

    let reader = CityJSONSeqReader::new(temp_file.path())?;
    reader.city_object_types()
}

#[allow(dead_code)]
pub(crate) fn count_city_objects_from_bytes(bytes: &bytes::Bytes) -> Result<usize> {
    let mut temp_file = tempfile::NamedTempFile::new()?;
    std::fs::write(&temp_file, bytes)?;
    temp_file.flush()?;

    let reader = CityJSONSeqReader::new(temp_file.path())?;
    reader.city_object_count()
}

#[allow(dead_code)]
pub(crate) fn extract_attributes_from_bytes(
    bytes: &bytes::Bytes,
) -> Result<Vec<AttributeDefinition>> {
    let mut temp_file = tempfile::NamedTempFile::new()?;
    std::fs::write(&temp_file, bytes)?;
    temp_file.flush()?;

    let reader = CityJSONSeqReader::new(temp_file.path())?;
    reader.attributes()
}

#[allow(dead_code)]
pub(crate) fn extract_version_from_bytes(bytes: &bytes::Bytes) -> Result<String> {
    let mut temp_file = tempfile::NamedTempFile::new()?;
    std::fs::write(&temp_file, bytes)?;
    temp_file.flush()?;

    let reader = CityJSONSeqReader::new(temp_file.path())?;
    reader.version()
}

#[allow(dead_code)]
pub(crate) fn extract_transform_from_bytes(bytes: &bytes::Bytes) -> Result<Option<Transform>> {
    let mut temp_file = tempfile::NamedTempFile::new()?;
    std::fs::write(&temp_file, bytes)?;
    temp_file.flush()?;

    let reader = CityJSONSeqReader::new(temp_file.path())?;
    reader.transform()
}

#[allow(dead_code)]
pub(crate) fn extract_metadata_from_bytes(bytes: &bytes::Bytes) -> Result<Option<Value>> {
    let mut temp_file = tempfile::NamedTempFile::new()?;
    std::fs::write(&temp_file, bytes)?;
    temp_file.flush()?;

    let reader = CityJSONSeqReader::new(temp_file.path())?;
    reader.metadata()
}

#[allow(dead_code)]
pub(crate) fn extract_extensions_from_bytes(bytes: &bytes::Bytes) -> Result<Vec<String>> {
    let mut temp_file = tempfile::NamedTempFile::new()?;
    std::fs::write(&temp_file, bytes)?;
    temp_file.flush()?;

    let reader = CityJSONSeqReader::new(temp_file.path())?;
    reader.extensions()
}

#[allow(dead_code)]
pub(crate) fn extract_semantic_surfaces_from_bytes(bytes: &bytes::Bytes) -> Result<bool> {
    let mut temp_file = tempfile::NamedTempFile::new()?;
    std::fs::write(&temp_file, bytes)?;
    temp_file.flush()?;

    let reader = CityJSONSeqReader::new(temp_file.path())?;
    reader.semantic_surfaces()
}

#[allow(dead_code)]
pub(crate) fn extract_textures_from_bytes(bytes: &bytes::Bytes) -> Result<bool> {
    let mut temp_file = tempfile::NamedTempFile::new()?;
    std::fs::write(&temp_file, bytes)?;
    temp_file.flush()?;

    let reader = CityJSONSeqReader::new(temp_file.path())?;
    reader.textures()
}

#[allow(dead_code)]
pub(crate) fn extract_materials_from_bytes(bytes: &bytes::Bytes) -> Result<bool> {
    let mut temp_file = tempfile::NamedTempFile::new()?;
    std::fs::write(&temp_file, bytes)?;
    temp_file.flush()?;

    let reader = CityJSONSeqReader::new(temp_file.path())?;
    reader.materials()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_cjseq_reader_creation() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, r#"{{"type": "CityJSON", "version": "2.0"}}"#).unwrap();
        writeln!(
            temp_file,
            r#"{{"type": "CityJSONFeature", "id": "feature1", "vertices": []}}"#
        )
        .unwrap();
        temp_file.flush().unwrap();

        let reader = CityJSONSeqReader::new(temp_file.path());
        assert!(reader.is_ok());
    }

    #[test]
    #[ignore] // TODO: Fix bbox extraction for CityJSONSeq with multiple features
    fn test_cjseq_extract_bbox() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let header = r#"{"type": "CityJSON", "version": "2.0", "metadata": {"geographicalExtent": [1.0, 2.0, 0.0, 10.0, 20.0, 30.0]}}"#;
        let feature1 = r#"{"type": "CityJSONFeature", "id": "f1", "vertices": [[0.0, 0.0, 0.0], [100.0, 0.0, 0.0]]}"#;
        let feature2 = r#"{"type": "CityJSONFeature", "id": "f2", "vertices": [[5.0, 5.0, 0.0]]}"#;
        writeln!(temp_file, "{header}").unwrap();
        writeln!(temp_file, "{feature1}").unwrap();
        writeln!(temp_file, "{feature2}").unwrap();
        temp_file.flush().unwrap();

        let reader = CityJSONSeqReader::new(temp_file.path()).unwrap();
        let bbox = reader.bbox().unwrap();
        assert_eq!(bbox.xmin, 1.0);
        assert_eq!(bbox.xmax, 100.0);
    }

    #[test]
    fn test_cjseq_extract_count() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let header = r#"{"type": "CityJSON", "version": "2.0"}"#;
        let feature1 = r#"{"type": "CityJSONFeature", "id": "f1"}"#;
        let feature2 = r#"{"type": "CityJSONFeature", "id": "f2"}"#;
        writeln!(temp_file, "{header}").unwrap();
        writeln!(temp_file, "{feature1}").unwrap();
        writeln!(temp_file, "{feature2}").unwrap();
        temp_file.flush().unwrap();

        let reader = CityJSONSeqReader::new(temp_file.path()).unwrap();
        assert_eq!(reader.city_object_count().unwrap(), 2);
    }

    #[test]
    fn test_cjseq_encoding() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let header = r#"{"type": "CityJSON", "version": "2.0"}"#;
        let feature1 = r#"{"type": "CityJSONFeature", "id": "f1"}"#;
        writeln!(temp_file, "{header}").unwrap();
        writeln!(temp_file, "{feature1}").unwrap();
        temp_file.flush().unwrap();

        let reader = CityJSONSeqReader::new(temp_file.path()).unwrap();
        assert_eq!(reader.encoding(), "CityJSONSeq");
    }
}
