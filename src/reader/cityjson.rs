//! CityJSON format reader
//!
//! This module provides a reader for CityJSON files (.json).
//!
//! Note: CityJSON files are not designed for streaming.
//! The CityJSONSeq format (.jsonl) is designed for streaming and has a
//! separate reader implementation.

use crate::error::{CityJsonStacError, Result};
use crate::metadata::{AttributeDefinition, AttributeType, BBox3D, Transform, CRS};
use crate::reader::CityModelMetadataReader;

//FIXME: don't use Value, use cjseq::CityJSON instead
use serde_json::Value;
use std::collections::{BTreeSet, HashMap};
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

/// Reader for CityJSON format files (.json)
///
/// Uses `RwLock` for interior mutability to enable lazy loading
/// while maintaining thread-safety (`Send + Sync` bounds).
pub struct CityJSONReader {
    file_path: PathBuf,
    /// Cached parsed CityJSON data (lazy loaded via interior mutability)
    // FIXME: don't use Value, use cjseq::CityJSON instead
    data: RwLock<Option<Value>>,
}

impl CityJSONReader {
    /// Create a new CityJSON reader
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

    /// Lazy load and cache CityJSON data using interior mutability
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
            // FIXME: don't use Value, use cjseq::CityJSON instead
            let value: Value = serde_json::from_reader(reader)?;
            *data = Some(value);
        }
        Ok(())
    }

    /// Execute a closure with access to the loaded data
    fn with_data<T, F>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&Value) -> Result<T>,
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

/// Extract bbox from CityJSON data
///
/// First tries to get from metadata.geographicalExtent,
/// then falls back to computing from vertices.
fn extract_bbox_from_data(data: &Value) -> Result<BBox3D> {
    // Try to get from metadata.geographicalExtent first
    if let Some(metadata) = data.get("metadata") {
        if let Some(extent) = metadata.get("geographicalExtent") {
            if let Some(arr) = extent.as_array() {
                if arr.len() >= 6 {
                    let vals: Vec<f64> = arr.iter().filter_map(|v| v.as_f64()).collect();
                    if vals.len() == 6 {
                        return Ok(BBox3D::new(
                            vals[0], vals[1], vals[2], vals[3], vals[4], vals[5],
                        ));
                    }
                }
            }
        }
    }

    // Fallback: compute from vertices
    if let Some(vertices) = data.get("vertices") {
        if let Some(verts_array) = vertices.as_array() {
            let mut xmin = f64::MAX;
            let mut ymin = f64::MAX;
            let mut zmin = f64::MAX;
            let mut xmax = f64::MIN;
            let mut ymax = f64::MIN;
            let mut zmax = f64::MIN;
            let mut found = false;

            for v in verts_array {
                if let Some(arr) = v.as_array() {
                    if arr.len() >= 3 {
                        if let (Some(x), Some(y), Some(z)) =
                            (arr[0].as_i64(), arr[1].as_i64(), arr[2].as_i64())
                        {
                            let xf = x as f64;
                            let yf = y as f64;
                            let zf = z as f64;
                            xmin = xmin.min(xf);
                            ymin = ymin.min(yf);
                            zmin = zmin.min(zf);
                            xmax = xmax.max(xf);
                            ymax = ymax.max(yf);
                            zmax = zmax.max(zf);
                            found = true;
                        }
                    }
                }
            }

            if found {
                return Ok(BBox3D::new(xmin, ymin, zmin, xmax, ymax, zmax));
            }
        }
    }

    // Default bbox if nothing found
    Ok(BBox3D::new(0.0, 0.0, 0.0, 0.0, 0.0, 0.0))
}

/// Extract CRS from CityJSON data
fn extract_crs_from_data(data: &Value) -> Result<CRS> {
    if let Some(metadata) = data.get("metadata") {
        // Try referenceSystem - can be object (CityJSON 1.1+) or string URL
        if let Some(rs) = metadata.get("referenceSystem") {
            // Handle as URL string: "https://www.opengis.net/def/crs/EPSG/0/7415"
            if let Some(rs_str) = rs.as_str() {
                // Extract EPSG code from URL like "https://www.opengis.net/def/crs/EPSG/0/7415"
                if rs_str.contains("EPSG") {
                    let parts: Vec<&str> = rs_str.split('/').collect();
                    if let Some(last) = parts.last() {
                        if let Ok(code) = last.parse::<u32>() {
                            return Ok(CRS::from_epsg(code));
                        }
                    }
                }
            }
            // Handle as object
            if let Some(rs_obj) = rs.as_object() {
                // Try to extract EPSG code from different possible fields
                if let Some(code) = rs_obj.get("code").and_then(|v| v.as_str()) {
                    if let Ok(epsg_code) = code.parse::<u32>() {
                        return Ok(CRS::from_epsg(epsg_code));
                    }
                }
                // Try referenceSystemName format like "EPSG:7415"
                if let Some(name) = rs_obj.get("referenceSystemName").and_then(|v| v.as_str()) {
                    if let Some(code_str) = name.strip_prefix("EPSG:") {
                        if let Ok(code) = code_str.parse::<u32>() {
                            return Ok(CRS::from_epsg(code));
                        }
                    }
                }
                // Try base_url field
                if let Some(base_url) = rs_obj.get("base_url").and_then(|v| v.as_str()) {
                    if base_url.contains("EPSG") {
                        let parts: Vec<&str> = base_url.split('/').collect();
                        if let Some(last) = parts.last() {
                            if let Ok(code) = last.parse::<u32>() {
                                return Ok(CRS::from_epsg(code));
                            }
                        }
                    }
                }
            }
        }

        // Fallback to CRS string (CityJSON 1.0)
        if let Some(crs_value) = metadata.get("CRS") {
            if let Some(crs_str) = crs_value.as_str() {
                // Handle EPSG:XXXX format
                if let Some(code_str) = crs_str.strip_prefix("EPSG:") {
                    if let Ok(code) = code_str.parse::<u32>() {
                        return Ok(CRS::from_epsg(code));
                    }
                }
                // Handle URN format
                if crs_str.contains("EPSG") {
                    let parts: Vec<&str> = crs_str.split('/').collect();
                    if let Some(last) = parts.last() {
                        if let Ok(code) = last.parse::<u32>() {
                            return Ok(CRS::from_epsg(code));
                        }
                    }
                }
            }
        }
    }

    // Default CRS
    Ok(CRS::default())
}

/// Extract transform from CityJSON data
fn extract_transform_from_data(data: &Value) -> Result<Option<Transform>> {
    if let Some(transform_obj) = data.get("transform") {
        if let Some(obj) = transform_obj.as_object() {
            let scale = obj.get("scale").and_then(|v| v.as_array()).and_then(|arr| {
                let vals: Vec<f64> = arr.iter().filter_map(|v| v.as_f64()).collect();
                if vals.len() == 3 {
                    Some([vals[0], vals[1], vals[2]])
                } else {
                    None
                }
            });

            let translate = obj
                .get("translate")
                .and_then(|v| v.as_array())
                .and_then(|arr| {
                    let vals: Vec<f64> = arr.iter().filter_map(|v| v.as_f64()).collect();
                    if vals.len() == 3 {
                        Some([vals[0], vals[1], vals[2]])
                    } else {
                        None
                    }
                });

            if let (Some(s), Some(t)) = (scale, translate) {
                return Ok(Some(Transform::new(s, t)));
            }
        }
    }
    Ok(None)
}

/// Extract LODs from CityJSON data
fn extract_lods_from_data(data: &Value) -> Result<Vec<String>> {
    let mut lods = BTreeSet::new();

    if let Some(city_objects) = data.get("CityObjects") {
        if let Some(objs) = city_objects.as_object() {
            for (_id, obj) in objs {
                if let Some(geometries) = obj.get("geometry") {
                    if let Some(geom_array) = geometries.as_array() {
                        for geom in geom_array {
                            if let Some(lod) = geom.get("lod").and_then(|v| v.as_str()) {
                                lods.insert(lod.to_string());
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(lods.into_iter().collect())
}

/// Extract city object types from CityJSON data
fn extract_city_object_types_from_data(data: &Value) -> Result<Vec<String>> {
    let mut types = BTreeSet::new();

    if let Some(city_objects) = data.get("CityObjects") {
        if let Some(objs) = city_objects.as_object() {
            for (_id, obj) in objs {
                if let Some(type_val) = obj.get("type") {
                    if let Some(type_str) = type_val.as_str() {
                        // Filter out extension types (starting with +)
                        if !type_str.starts_with('+') {
                            types.insert(type_str.to_string());
                        }
                    }
                }
            }
        }
    }

    Ok(types.into_iter().collect())
}

/// Extract attributes from CityJSON data
fn extract_attributes_from_data(data: &Value) -> Result<Vec<AttributeDefinition>> {
    let mut attributes_map: HashMap<String, AttributeType> = HashMap::new();

    if let Some(city_objects) = data.get("CityObjects") {
        if let Some(objs) = city_objects.as_object() {
            for (_id, obj) in objs {
                if let Some(attrs) = obj.get("attributes") {
                    if let Some(attrs_obj) = attrs.as_object() {
                        for (attr_name, attr_value) in attrs_obj {
                            let attr_type = match attr_value {
                                Value::String(_) => AttributeType::String,
                                Value::Number(_) => AttributeType::Number,
                                Value::Bool(_) => AttributeType::Boolean,
                                Value::Array(_) => AttributeType::Array,
                                Value::Object(_) => AttributeType::Object,
                                Value::Null => continue,
                            };

                            // Merge attribute types: if conflicting types, use String
                            attributes_map
                                .entry(attr_name.clone())
                                .and_modify(|existing| {
                                    if *existing != attr_type {
                                        *existing = AttributeType::String;
                                    }
                                })
                                .or_insert(attr_type);
                        }
                    }
                }
            }
        }
    }

    let mut attributes: Vec<_> = attributes_map
        .into_iter()
        .map(|(name, attr_type)| AttributeDefinition::new(&name, attr_type))
        .collect();

    // Sort by name for consistent output
    attributes.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(attributes)
}

/// Extract extensions from CityJSON data
fn extract_extensions_from_data(data: &Value) -> Result<Vec<String>> {
    let mut extensions = Vec::new();

    if let Some(ext) = data.get("extensions") {
        if let Some(ext_obj) = ext.as_object() {
            for (url, _name) in ext_obj {
                extensions.push(url.clone());
            }
        }
    }

    extensions.sort();
    Ok(extensions)
}

/// Extract semantic surfaces presence from CityJSON data
fn extract_semantic_surfaces_from_data(data: &Value) -> Result<bool> {
    if let Some(city_objects) = data.get("CityObjects") {
        if let Some(objs) = city_objects.as_object() {
            for (_id, obj) in objs {
                if let Some(geometries) = obj.get("geometry") {
                    if let Some(geom_array) = geometries.as_array() {
                        for geom in geom_array {
                            if geom.get("semantics").is_some() {
                                return Ok(true);
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(false)
}

/// Extract textures presence from CityJSON data
fn extract_textures_from_data(data: &Value) -> Result<bool> {
    if let Some(appearance) = data.get("appearance") {
        if appearance.get("textures").is_some() {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Extract materials presence from CityJSON data
fn extract_materials_from_data(data: &Value) -> Result<bool> {
    if let Some(appearance) = data.get("appearance") {
        if appearance.get("materials").is_some() {
            return Ok(true);
        }
    }
    Ok(false)
}

impl CityModelMetadataReader for CityJSONReader {
    fn bbox(&self) -> Result<BBox3D> {
        self.with_data(extract_bbox_from_data)
    }

    fn crs(&self) -> Result<CRS> {
        self.with_data(extract_crs_from_data)
    }

    fn lods(&self) -> Result<Vec<String>> {
        self.with_data(extract_lods_from_data)
    }

    fn city_object_types(&self) -> Result<Vec<String>> {
        self.with_data(extract_city_object_types_from_data)
    }

    fn city_object_count(&self) -> Result<usize> {
        self.with_data(|data| {
            if let Some(city_objects) = data.get("CityObjects") {
                if let Some(objs) = city_objects.as_object() {
                    return Ok(objs.len());
                }
            }
            Ok(0)
        })
    }

    fn attributes(&self) -> Result<Vec<AttributeDefinition>> {
        self.with_data(extract_attributes_from_data)
    }

    fn encoding(&self) -> &'static str {
        "CityJSON"
    }

    fn version(&self) -> Result<String> {
        self.with_data(|data| {
            if let Some(version) = data.get("version") {
                if let Some(v) = version.as_str() {
                    return Ok(v.to_string());
                }
            }
            Ok("1.0".to_string()) // Default version
        })
    }

    fn file_path(&self) -> &Path {
        &self.file_path
    }

    fn transform(&self) -> Result<Option<Transform>> {
        self.with_data(extract_transform_from_data)
    }

    fn metadata(&self) -> Result<Option<Value>> {
        self.with_data(|data| Ok(data.get("metadata").cloned()))
    }

    fn extensions(&self) -> Result<Vec<String>> {
        self.with_data(extract_extensions_from_data)
    }

    fn semantic_surfaces(&self) -> Result<bool> {
        self.with_data(extract_semantic_surfaces_from_data)
    }

    fn textures(&self) -> Result<bool> {
        self.with_data(extract_textures_from_data)
    }

    fn materials(&self) -> Result<bool> {
        self.with_data(extract_materials_from_data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn create_test_cityjson() -> NamedTempFile {
        let mut temp_file = NamedTempFile::new().unwrap();
        let cityjson = r#"{
            "type": "CityJSON",
            "version": "2.0",
            "metadata": {
                "geographicalExtent": [1.0, 2.0, 0.0, 10.0, 20.0, 30.0],
                "referenceSystem": {
                    "type": "referenceSystem",
                    "referenceSystemName": "EPSG:7415",
                    "base_url": "https://www.opengis.net/def/crs/EPSG/0/7415",
                    "authority": "EPSG",
                    "version": "0",
                    "code": "7415"
                }
            },
            "CityObjects": {
                "building1": {
                    "type": "Building",
                    "geometry": [{
                        "type": "Solid",
                        "lod": "2",
                        "boundaries": []
                    }],
                    "attributes": {
                        "yearOfConstruction": 2020,
                        "function": "residential"
                    }
                },
                "building2": {
                    "type": "Building",
                    "geometry": [{
                        "type": "Solid",
                        "lod": "2.2",
                        "boundaries": []
                    }],
                    "attributes": {
                        "yearOfConstruction": 2021
                    }
                }
            },
            "vertices": []
        }"#;

        writeln!(temp_file, "{}", cityjson).unwrap();
        temp_file.flush().unwrap();
        temp_file
    }

    #[test]
    fn test_cityjson_reader_creation() {
        let temp_file = create_test_cityjson();
        let reader = CityJSONReader::new(temp_file.path());
        assert!(reader.is_ok());
    }

    #[test]
    fn test_cityjson_reader_not_found() {
        let reader = CityJSONReader::new(Path::new("/nonexistent/file.json"));
        assert!(reader.is_err());
    }

    #[test]
    fn test_cityjson_extract_version() {
        let temp_file = create_test_cityjson();
        let reader = CityJSONReader::new(temp_file.path()).unwrap();
        let version = reader.version().unwrap();
        assert_eq!(version, "2.0");
    }

    #[test]
    fn test_cityjson_extract_city_objects_count() {
        let temp_file = create_test_cityjson();
        let reader = CityJSONReader::new(temp_file.path()).unwrap();
        let count = reader.city_object_count().unwrap();
        assert_eq!(count, 2);
    }

    #[test]
    fn test_cityjson_extract_types() {
        let temp_file = create_test_cityjson();
        let reader = CityJSONReader::new(temp_file.path()).unwrap();
        let types = reader.city_object_types().unwrap();
        assert_eq!(types, vec!["Building"]);
    }

    #[test]
    fn test_cityjson_extract_lods() {
        let temp_file = create_test_cityjson();
        let reader = CityJSONReader::new(temp_file.path()).unwrap();
        let lods = reader.lods().unwrap();
        assert!(lods.contains(&"2".to_string()));
        assert!(lods.contains(&"2.2".to_string()));
    }

    #[test]
    fn test_cityjson_extract_bbox() {
        let temp_file = create_test_cityjson();
        let reader = CityJSONReader::new(temp_file.path()).unwrap();
        let bbox = reader.bbox().unwrap();
        assert_eq!(bbox.xmin, 1.0);
        assert_eq!(bbox.ymin, 2.0);
        assert_eq!(bbox.xmax, 10.0);
        assert_eq!(bbox.ymax, 20.0);
    }

    #[test]
    fn test_cityjson_extract_crs() {
        let temp_file = create_test_cityjson();
        let reader = CityJSONReader::new(temp_file.path()).unwrap();
        let crs = reader.crs().unwrap();
        assert_eq!(crs.epsg, Some(7415));
    }

    #[test]
    fn test_cityjson_extract_attributes() {
        let temp_file = create_test_cityjson();
        let reader = CityJSONReader::new(temp_file.path()).unwrap();
        let attrs = reader.attributes().unwrap();

        let attr_names: Vec<&str> = attrs.iter().map(|a| a.name.as_str()).collect();
        assert!(attr_names.contains(&"yearOfConstruction"));
        assert!(attr_names.contains(&"function"));
    }

    #[test]
    fn test_cityjson_encoding() {
        let temp_file = create_test_cityjson();
        let reader = CityJSONReader::new(temp_file.path()).unwrap();
        assert_eq!(reader.encoding(), "CityJSON");
    }

    #[test]
    fn test_cityjson_extensions_empty() {
        // Standard test file has no extensions
        let temp_file = create_test_cityjson();
        let reader = CityJSONReader::new(temp_file.path()).unwrap();
        let extensions = reader.extensions().unwrap();
        assert!(extensions.is_empty());
    }

    #[test]
    fn test_cityjson_extensions_present() {
        // Create a test file with extensions
        let mut temp_file = NamedTempFile::new().unwrap();
        let cityjson = r#"{
            "type": "CityJSON",
            "version": "2.0",
            "extensions": {
                "https://www.cityjson.org/extensions/noise.ext.json": "Noise",
                "https://3dbag.nl/extensions/3dbag.ext.json": "3DBAG"
            },
            "metadata": {
                "geographicalExtent": [1.0, 2.0, 0.0, 10.0, 20.0, 30.0]
            },
            "CityObjects": {
                "building1": {
                    "type": "+NoiseBuilding",
                    "geometry": []
                }
            },
            "vertices": []
        }"#;

        writeln!(temp_file, "{}", cityjson).unwrap();
        let reader = CityJSONReader::new(temp_file.path()).unwrap();
        let extensions = reader.extensions().unwrap();

        assert_eq!(extensions.len(), 2);
        assert!(
            extensions.contains(&"https://www.cityjson.org/extensions/noise.ext.json".to_string())
        );
        assert!(extensions.contains(&"https://3dbag.nl/extensions/3dbag.ext.json".to_string()));
    }

    #[test]
    fn test_cityjson_extensions_sorted() {
        // Extensions should be returned sorted
        let mut temp_file = NamedTempFile::new().unwrap();
        let cityjson = r#"{
            "type": "CityJSON",
            "version": "2.0",
            "extensions": {
                "https://z.ext.json": "Z",
                "https://a.ext.json": "A"
            },
            "metadata": {
                "geographicalExtent": [0, 0, 0, 1, 1, 1]
            },
            "CityObjects": {},
            "vertices": []
        }"#;

        writeln!(temp_file, "{}", cityjson).unwrap();
        let reader = CityJSONReader::new(temp_file.path()).unwrap();
        let extensions = reader.extensions().unwrap();

        // Check that extensions are sorted
        assert_eq!(extensions[0], "https://a.ext.json");
        assert_eq!(extensions[1], "https://z.ext.json");
    }

    #[test]
    fn test_cityjson_semantic_surfaces() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let cityjson = r#"{
            "type": "CityJSON",
            "version": "2.0",
            "metadata": {
                "geographicalExtent": [0, 0, 0, 1, 1, 1]
            },
            "CityObjects": {
                "building1": {
                    "type": "Building",
                    "geometry": [{
                        "type": "Solid",
                        "lod": "2",
                        "boundaries": [],
                        "semantics": {
                            "surfaces": [
                                {"type": "Wall"},
                                {"type": "Roof"}
                            ]
                        }
                    }]
                }
            },
            "vertices": []
        }"#;

        writeln!(temp_file, "{}", cityjson).unwrap();
        let reader = CityJSONReader::new(temp_file.path()).unwrap();
        assert!(reader.semantic_surfaces().unwrap());
    }

    #[test]
    fn test_cityjson_no_semantic_surfaces() {
        let temp_file = create_test_cityjson();
        let reader = CityJSONReader::new(temp_file.path()).unwrap();
        assert!(!reader.semantic_surfaces().unwrap());
    }

    #[test]
    fn test_cityjson_textures() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let cityjson = r#"{
            "type": "CityJSON",
            "version": "2.0",
            "metadata": {
                "geographicalExtent": [0, 0, 0, 1, 1, 1]
            },
            "appearance": {
                "textures": [
                    {
                        "type": "PNG",
                        "image": "base64..."
                    }
                ]
            },
            "CityObjects": {},
            "vertices": []
        }"#;

        writeln!(temp_file, "{}", cityjson).unwrap();
        let reader = CityJSONReader::new(temp_file.path()).unwrap();
        assert!(reader.textures().unwrap());
    }

    #[test]
    fn test_cityjson_materials() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let cityjson = r#"{
            "type": "CityJSON",
            "version": "2.0",
            "metadata": {
                "geographicalExtent": [0, 0, 0, 1, 1, 1]
            },
            "appearance": {
                "materials": [
                    {
                        "name": "roof",
                        "ambientIntensity": 0.6
                    }
                ]
            },
            "CityObjects": {},
            "vertices": []
        }"#;

        writeln!(temp_file, "{}", cityjson).unwrap();
        let reader = CityJSONReader::new(temp_file.path()).unwrap();
        assert!(reader.materials().unwrap());
    }

    #[test]
    fn test_cityjson_transform() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let cityjson = r#"{
            "type": "CityJSON",
            "version": "2.0",
            "transform": {
                "scale": [0.01, 0.01, 0.01],
                "translate": [100000, 200000, 0]
            },
            "metadata": {
                "geographicalExtent": [0, 0, 0, 1, 1, 1]
            },
            "CityObjects": {},
            "vertices": []
        }"#;

        writeln!(temp_file, "{}", cityjson).unwrap();
        let reader = CityJSONReader::new(temp_file.path()).unwrap();
        let transform = reader.transform().unwrap();
        assert!(transform.is_some());

        let t = transform.unwrap();
        assert_eq!(t.scale, [0.01, 0.01, 0.01]);
        assert_eq!(t.translate, [100000.0, 200000.0, 0.0]);
    }

    #[test]
    fn test_cityjson_metadata() {
        let temp_file = create_test_cityjson();
        let reader = CityJSONReader::new(temp_file.path()).unwrap();
        let metadata = reader.metadata().unwrap();
        assert!(metadata.is_some());
    }
}
