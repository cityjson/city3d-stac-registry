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
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::sync::RwLock;

/// Data structure to hold parsed CityJSONSeq content
struct CityJSONSeqData {
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

    /// Lazy load and cache the CityJSONSeq data using interior mutability
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

            // Parse the first line as the header (CityJSON object)
            let header_line = lines.next().ok_or_else(|| {
                CityJsonStacError::InvalidCityJson("Empty CityJSONSeq file".to_string())
            })??;

            let header: Value = serde_json::from_str(&header_line)?;

            // Validate that the first line is a CityJSON header
            let obj_type = header.get("type").and_then(|t| t.as_str());
            if obj_type != Some("CityJSON") {
                return Err(CityJsonStacError::InvalidCityJson(
                    "First line of CityJSONSeq must be a CityJSON object".to_string(),
                ));
            }

            // Parse remaining lines as CityJSONFeature objects
            let mut features = Vec::new();
            for (line_num, line_result) in lines.enumerate() {
                let line = line_result?;
                if line.trim().is_empty() {
                    continue;
                }

                let feature: Value = serde_json::from_str(&line).map_err(|e| {
                    CityJsonStacError::InvalidCityJson(format!(
                        "Failed to parse feature at line {}: {}",
                        line_num + 2,
                        e
                    ))
                })?;

                // Validate that it's a CityJSONFeature
                let feat_type = feature.get("type").and_then(|t| t.as_str());
                if feat_type != Some("CityJSONFeature") {
                    return Err(CityJsonStacError::InvalidCityJson(format!(
                        "Line {} is not a CityJSONFeature (got type: {:?})",
                        line_num + 2,
                        feat_type
                    )));
                }

                features.push(feature);
            }

            *data = Some(CityJSONSeqData { header, features });
        }
        Ok(())
    }

    /// Execute a closure with access to the loaded data
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

// Helper function to extract f64 from JSON array with proper error handling
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
fn extract_transform_from_header(header: &Value) -> Result<Option<Transform>> {
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
/// First tries the header's geographicalExtent, then computes from all features
fn extract_bbox_from_data(data: &CityJSONSeqData) -> Result<BBox3D> {
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

    if xmin == f64::MAX {
        return Err(CityJsonStacError::MetadataError(
            "Could not determine bounding box from CityJSONSeq".to_string(),
        ));
    }

    Ok(BBox3D::new(xmin, ymin, zmin, xmax, ymax, zmax))
}

/// Extract CRS from header
fn extract_crs_from_header(header: &Value) -> Result<CRS> {
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

/// Extract LODs from all features (uses BTreeSet for automatic sorting)
fn extract_lods_from_features(features: &[Value]) -> Result<Vec<String>> {
    let mut lods = BTreeSet::new();

    for feature in features {
        if let Some(city_objects) = feature.get("CityObjects") {
            if let Some(objects) = city_objects.as_object() {
                for (_id, obj) in objects {
                    if let Some(geometry) = obj.get("geometry") {
                        if let Some(geom_array) = geometry.as_array() {
                            for geom in geom_array {
                                if let Some(lod) = geom.get("lod") {
                                    if let Some(lod_num) = lod.as_f64() {
                                        lods.insert(format!("{}", lod_num));
                                    } else if let Some(lod_str) = lod.as_str() {
                                        lods.insert(lod_str.to_string());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(lods.into_iter().collect())
}

/// Extract city object types from all features (uses BTreeSet for automatic sorting)
fn extract_city_object_types_from_features(features: &[Value]) -> Result<Vec<String>> {
    let mut types = BTreeSet::new();

    for feature in features {
        if let Some(city_objects) = feature.get("CityObjects") {
            if let Some(objects) = city_objects.as_object() {
                for (_id, obj) in objects {
                    if let Some(obj_type) = obj.get("type") {
                        if let Some(type_str) = obj_type.as_str() {
                            types.insert(type_str.to_string());
                        }
                    }
                }
            }
        }
    }

    Ok(types.into_iter().collect())
}

/// Count total city objects across all features
fn count_city_objects_from_features(features: &[Value]) -> Result<usize> {
    let mut count = 0;

    for feature in features {
        if let Some(city_objects) = feature.get("CityObjects") {
            if let Some(objects) = city_objects.as_object() {
                count += objects.len();
            }
        }
    }

    Ok(count)
}

/// Extract attribute schema from all features
fn extract_attributes_from_features(features: &[Value]) -> Result<Vec<AttributeDefinition>> {
    let mut attribute_map: HashMap<String, AttributeType> = HashMap::new();

    for feature in features {
        if let Some(city_objects) = feature.get("CityObjects") {
            if let Some(objects) = city_objects.as_object() {
                for (_id, obj) in objects {
                    if let Some(attributes) = obj.get("attributes") {
                        if let Some(attrs) = attributes.as_object() {
                            for (key, value) in attrs {
                                let attr_type = AttributeType::from_json_value(value);
                                attribute_map.insert(key.clone(), attr_type);
                            }
                        }
                    }
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

/// Extract version from header
fn extract_version_from_header(header: &Value) -> Result<String> {
    if let Some(version) = header.get("version") {
        if let Some(v_str) = version.as_str() {
            return Ok(v_str.to_string());
        }
    }

    Ok("2.0".to_string()) // Default version for CityJSONSeq (requires CityJSON 2.0+)
}

/// Extract CityJSON extensions from header (Application Domain Extensions)
///
/// In CityJSONSeq, extensions are declared in the header (first line).
fn extract_extensions_from_header(header: &Value) -> Result<Vec<String>> {
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
        self.with_data(|data| count_city_objects_from_features(&data.features))
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
        self.with_data(|data| Ok(data.header.get("metadata").cloned()))
    }

    fn extensions(&self) -> Result<Vec<String>> {
        self.with_data(|data| extract_extensions_from_header(&data.header))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn create_test_cjseq() -> NamedTempFile {
        let mut temp_file = tempfile::Builder::new()
            .suffix(".city.jsonl")
            .tempfile()
            .unwrap();

        // Write header line (CityJSON object)
        writeln!(
            temp_file,
            r#"{{"type":"CityJSON","version":"2.0","transform":{{"scale":[0.001,0.001,0.001],"translate":[0,0,0]}},"CityObjects":{{}},"vertices":[],"metadata":{{"geographicalExtent":[1.0,2.0,0.0,10.0,20.0,30.0],"referenceSystem":"https://www.opengis.net/def/crs/EPSG/0/7415"}}}}"#
        )
        .unwrap();

        // Write first feature
        writeln!(
            temp_file,
            r#"{{"type":"CityJSONFeature","id":"building1","CityObjects":{{"building1":{{"type":"Building","geometry":[{{"type":"Solid","lod":"2","boundaries":[]}}],"attributes":{{"yearOfConstruction":2020,"function":"residential"}}}}}},"vertices":[[1000,2000,0],[10000,20000,30000]]}}"#
        )
        .unwrap();

        // Write second feature
        writeln!(
            temp_file,
            r#"{{"type":"CityJSONFeature","id":"building2","CityObjects":{{"building2":{{"type":"Building","geometry":[{{"type":"Solid","lod":"2.2","boundaries":[]}}],"attributes":{{"yearOfConstruction":2021}}}}}},"vertices":[[2000,3000,1000]]}}"#
        )
        .unwrap();

        temp_file.flush().unwrap();
        temp_file
    }

    #[test]
    fn test_cjseq_reader_creation() {
        let temp_file = create_test_cjseq();
        let reader = CityJSONSeqReader::new(temp_file.path());
        assert!(reader.is_ok());
    }

    #[test]
    fn test_cjseq_extract_version() {
        let temp_file = create_test_cjseq();
        let reader = CityJSONSeqReader::new(temp_file.path()).unwrap();
        let version = reader.version().unwrap();
        assert_eq!(version, "2.0");
    }

    #[test]
    fn test_cjseq_extract_city_objects_count() {
        let temp_file = create_test_cjseq();
        let reader = CityJSONSeqReader::new(temp_file.path()).unwrap();
        let count = reader.city_object_count().unwrap();
        assert_eq!(count, 2);
    }

    #[test]
    fn test_cjseq_extract_types() {
        let temp_file = create_test_cjseq();
        let reader = CityJSONSeqReader::new(temp_file.path()).unwrap();
        let types = reader.city_object_types().unwrap();
        assert_eq!(types, vec!["Building"]);
    }

    #[test]
    fn test_cjseq_extract_lods() {
        let temp_file = create_test_cjseq();
        let reader = CityJSONSeqReader::new(temp_file.path()).unwrap();
        let lods = reader.lods().unwrap();
        assert!(lods.contains(&"2".to_string()));
        assert!(lods.contains(&"2.2".to_string()));
    }

    #[test]
    fn test_cjseq_extract_bbox() {
        let temp_file = create_test_cjseq();
        let reader = CityJSONSeqReader::new(temp_file.path()).unwrap();
        let bbox = reader.bbox().unwrap();
        // From metadata.geographicalExtent in header
        assert_eq!(bbox.xmin, 1.0);
        assert_eq!(bbox.ymin, 2.0);
        assert_eq!(bbox.xmax, 10.0);
        assert_eq!(bbox.ymax, 20.0);
    }

    #[test]
    fn test_cjseq_extract_crs() {
        let temp_file = create_test_cjseq();
        let reader = CityJSONSeqReader::new(temp_file.path()).unwrap();
        let crs = reader.crs().unwrap();
        assert_eq!(crs.epsg, Some(7415));
    }

    #[test]
    fn test_cjseq_extract_attributes() {
        let temp_file = create_test_cjseq();
        let reader = CityJSONSeqReader::new(temp_file.path()).unwrap();
        let attrs = reader.attributes().unwrap();

        let attr_names: Vec<&str> = attrs.iter().map(|a| a.name.as_str()).collect();
        assert!(attr_names.contains(&"yearOfConstruction"));
        assert!(attr_names.contains(&"function"));
    }

    #[test]
    fn test_cjseq_encoding() {
        let temp_file = create_test_cjseq();
        let reader = CityJSONSeqReader::new(temp_file.path()).unwrap();
        assert_eq!(reader.encoding(), "CityJSONSeq");
    }

    #[test]
    fn test_cjseq_transform() {
        let temp_file = create_test_cjseq();
        let reader = CityJSONSeqReader::new(temp_file.path()).unwrap();
        let transform = reader.transform().unwrap();
        assert!(transform.is_some());
        let t = transform.unwrap();
        assert_eq!(t.scale, [0.001, 0.001, 0.001]);
        assert_eq!(t.translate, [0.0, 0.0, 0.0]);
    }

    #[test]
    fn test_cjseq_invalid_header_type() {
        let mut temp_file = tempfile::Builder::new()
            .suffix(".city.jsonl")
            .tempfile()
            .unwrap();
        // Write invalid header (wrong type)
        writeln!(temp_file, r#"{{"type":"Invalid","version":"2.0"}}"#).unwrap();
        temp_file.flush().unwrap();

        let reader = CityJSONSeqReader::new(temp_file.path()).unwrap();
        // Trigger lazy load
        let result = reader.bbox();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("First line"));
    }

    #[test]
    fn test_cjseq_malformed_json_header() {
        let mut temp_file = tempfile::Builder::new()
            .suffix(".city.jsonl")
            .tempfile()
            .unwrap();
        writeln!(temp_file, r#"{{ not json }}"#).unwrap();
        temp_file.flush().unwrap();

        let reader = CityJSONSeqReader::new(temp_file.path()).unwrap();
        let result = reader.version();
        assert!(result.is_err());
    }

    #[test]
    fn test_cjseq_invalid_feature_type() {
        let mut temp_file = tempfile::Builder::new()
            .suffix(".city.jsonl")
            .tempfile()
            .unwrap();
        // Valid header
        writeln!(temp_file, r#"{{"type":"CityJSON","version":"2.0","transform":{{"scale":[0.001,0.001,0.001],"translate":[0,0,0]}},"CityObjects":{{}},"vertices":[]}}"#).unwrap();
        // Invalid feature
        writeln!(temp_file, r#"{{"type":"NotAFeature"}}"#).unwrap();
        temp_file.flush().unwrap();

        let reader = CityJSONSeqReader::new(temp_file.path()).unwrap();
        let result = reader.city_object_count();
        assert!(result.is_err());
    }

    #[test]
    fn test_cjseq_extract_bbox_fallback() {
        let mut temp_file = tempfile::Builder::new()
            .suffix(".city.jsonl")
            .tempfile()
            .unwrap();
        // Write header without geographicalExtent
        // transform: scale=0.001, translate=[0,0,0]
        writeln!(
            temp_file,
            r#"{{"type":"CityJSON","version":"2.0","transform":{{"scale":[0.001,0.001,0.001],"translate":[0,0,0]}},"CityObjects":{{}},"vertices":[]}}"#
        )
        .unwrap();

        // Write feature with vertices
        // [1000, 2000, 0] -> [1.0, 2.0, 0.0]
        // [10000, 20000, 30000] -> [10.0, 20.0, 30.0]
        writeln!(
            temp_file,
            r#"{{"type":"CityJSONFeature","id":"b1","CityObjects":{{}},"vertices":[[1000,2000,0],[10000,20000,30000]]}}"#
        )
        .unwrap();

        temp_file.flush().unwrap();

        let reader = CityJSONSeqReader::new(temp_file.path()).unwrap();
        let bbox = reader.bbox().unwrap();

        assert!((bbox.xmin - 1.0).abs() < 1e-6);
        assert!((bbox.ymin - 2.0).abs() < 1e-6);
        assert!((bbox.zmin - 0.0).abs() < 1e-6);
        assert!((bbox.xmax - 10.0).abs() < 1e-6);
        assert!((bbox.ymax - 20.0).abs() < 1e-6);
        assert!((bbox.zmax - 30.0).abs() < 1e-6);
    }

    #[test]
    fn test_cjseq_extensions_empty() {
        // Standard test file has no extensions
        let temp_file = create_test_cjseq();
        let reader = CityJSONSeqReader::new(temp_file.path()).unwrap();
        let extensions = reader.extensions().unwrap();
        assert!(extensions.is_empty());
    }

    #[test]
    fn test_cjseq_extensions_present() {
        // Create a test file with extensions in header
        let mut temp_file = tempfile::Builder::new()
            .suffix(".city.jsonl")
            .tempfile()
            .unwrap();

        // Write header line with extensions
        writeln!(
            temp_file,
            r#"{{"type":"CityJSON","version":"2.0","extensions":{{"https://www.cityjson.org/extensions/noise.ext.json":"Noise"}},"transform":{{"scale":[0.001,0.001,0.001],"translate":[0,0,0]}},"CityObjects":{{}},"vertices":[]}}"#
        )
        .unwrap();

        // Write a feature
        writeln!(
            temp_file,
            r#"{{"type":"CityJSONFeature","id":"building1","CityObjects":{{"building1":{{"type":"+NoiseBuilding","geometry":[]}}}},"vertices":[]}}"#
        )
        .unwrap();

        temp_file.flush().unwrap();

        let reader = CityJSONSeqReader::new(temp_file.path()).unwrap();
        let extensions = reader.extensions().unwrap();

        assert_eq!(extensions.len(), 1);
        assert!(
            extensions.contains(&"https://www.cityjson.org/extensions/noise.ext.json".to_string())
        );
    }
}
