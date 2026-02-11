//! CityJSON format reader

use crate::error::{CityJsonStacError, Result};
use crate::metadata::{AttributeDefinition, AttributeType, BBox3D, Transform, CRS};
use crate::reader::CityModelMetadataReader;
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
    /// Cached parsed data (lazy loaded via interior mutability)
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

    /// Lazy load and cache JSON data using interior mutability
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
            *data = Some(serde_json::from_reader(reader)?);
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

// Helper function to extract bbox from data
fn extract_bbox_from_data(data: &Value) -> Result<BBox3D> {
    // Try to get from metadata.geographicalExtent first
    if let Some(metadata) = data.get("metadata") {
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

    // Extract transform for vertex processing
    let transform = extract_transform_from_data(data)?;

    // Fallback: compute from vertices
    if let Some(vertices) = data.get("vertices") {
        if let Some(vertex_array) = vertices.as_array() {
            if vertex_array.is_empty() {
                return Err(CityJsonStacError::MetadataError(
                    "No vertices found".to_string(),
                ));
            }

            let mut xmin = f64::MAX;
            let mut ymin = f64::MAX;
            let mut zmin = f64::MAX;
            let mut xmax = f64::MIN;
            let mut ymax = f64::MIN;
            let mut zmax = f64::MIN;

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

            return Ok(BBox3D::new(xmin, ymin, zmin, xmax, ymax, zmax));
        }
    }

    Err(CityJsonStacError::MetadataError(
        "Could not determine bounding box".to_string(),
    ))
}

// Helper function to extract CRS from data
fn extract_crs_from_data(data: &Value) -> Result<CRS> {
    if let Some(metadata) = data.get("metadata") {
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

// Helper function to extract transform from data
fn extract_transform_from_data(data: &Value) -> Result<Option<Transform>> {
    if let Some(transform) = data.get("transform") {
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

// Helper function to extract LODs from data (uses BTreeSet for automatic sorting)
fn extract_lods_from_data(data: &Value) -> Result<Vec<String>> {
    let mut lods = BTreeSet::new();

    if let Some(city_objects) = data.get("CityObjects") {
        if let Some(objects) = city_objects.as_object() {
            for (_id, obj) in objects {
                if let Some(geometry) = obj.get("geometry") {
                    if let Some(geom_array) = geometry.as_array() {
                        for geom in geom_array {
                            if let Some(lod) = geom.get("lod") {
                                if let Some(lod_num) = lod.as_f64() {
                                    lods.insert(format!("{lod_num}"));
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

    // BTreeSet is already sorted, just collect to Vec
    Ok(lods.into_iter().collect())
}

// Helper function to extract city object types (uses BTreeSet for automatic sorting)
fn extract_city_object_types_from_data(data: &Value) -> Result<Vec<String>> {
    let mut types = BTreeSet::new();

    if let Some(city_objects) = data.get("CityObjects") {
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

    // BTreeSet is already sorted, just collect to Vec
    Ok(types.into_iter().collect())
}

// Helper function to count city objects
fn count_city_objects_from_data(data: &Value) -> Result<usize> {
    if let Some(city_objects) = data.get("CityObjects") {
        if let Some(objects) = city_objects.as_object() {
            return Ok(objects.len());
        }
    }

    Ok(0)
}

// Helper function to extract attribute schema
fn extract_attributes_from_data(data: &Value) -> Result<Vec<AttributeDefinition>> {
    let mut attribute_map: HashMap<String, AttributeType> = HashMap::new();

    if let Some(city_objects) = data.get("CityObjects") {
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

    let mut attributes: Vec<AttributeDefinition> = attribute_map
        .into_iter()
        .map(|(name, attr_type)| AttributeDefinition::new(name, attr_type))
        .collect();

    attributes.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(attributes)
}

// Helper function to extract version
fn extract_version_from_data(data: &Value) -> Result<String> {
    if let Some(version) = data.get("version") {
        if let Some(v_str) = version.as_str() {
            return Ok(v_str.to_string());
        }
    }

    Ok("1.0".to_string()) // Default version
}

/// Helper function to extract CityJSON extensions (Application Domain Extensions)
///
/// In CityJSON, extensions are declared at the root level in an object where:
/// - The key is the URL to the extension schema file
/// - The value is the extension name/prefix (used for new City Object types with "+" prefix)
///
/// Example:
/// ```json
/// "extensions": {
///   "https://example.org/noise.ext.json": "Noise",
///   "https://cityjson.org/extensions/3dbag.json": "3DBAG"
/// }
/// ```
fn extract_extensions_from_data(data: &Value) -> Result<Vec<String>> {
    let mut extensions = Vec::new();

    if let Some(ext_obj) = data.get("extensions") {
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
        self.with_data(count_city_objects_from_data)
    }

    fn attributes(&self) -> Result<Vec<AttributeDefinition>> {
        self.with_data(extract_attributes_from_data)
    }

    fn encoding(&self) -> &'static str {
        "CityJSON"
    }

    fn version(&self) -> Result<String> {
        self.with_data(extract_version_from_data)
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
        self.with_data(|data| {
            // Check if any geometry has semantic surfaces
            // CityJSON 2.0 stores semantic surfaces in the "semantics" object
            // which is present alongside geometry boundaries
            if let Some(city_objects) = data.get("CityObjects") {
                if let Some(objects) = city_objects.as_object() {
                    for (_id, obj) in objects {
                        if let Some(geometry) = obj.get("geometry") {
                            if let Some(geom_array) = geometry.as_array() {
                                for geom in geom_array {
                                    // Check for semantics property which indicates semantic surfaces
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
        })
    }

    fn textures(&self) -> Result<bool> {
        self.with_data(|data| {
            // Check for textures in the root-level textures object
            Ok(data.get("textures").is_some())
        })
    }

    fn materials(&self) -> Result<bool> {
        self.with_data(|data| {
            // Check for materials in the root-level materials object
            Ok(data.get("materials").is_some())
        })
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
                "referenceSystem": "https://www.opengis.net/def/crs/EPSG/0/7415"
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
                "geographicalExtent": [1.0, 2.0, 0.0, 10.0, 20.0, 30.0],
                "referenceSystem": "https://www.opengis.net/def/crs/EPSG/0/7415"
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
        temp_file.flush().unwrap();

        let reader = CityJSONReader::new(temp_file.path()).unwrap();
        let extensions = reader.extensions().unwrap();

        assert_eq!(extensions.len(), 2);
        assert!(extensions.contains(&"https://3dbag.nl/extensions/3dbag.ext.json".to_string()));
        assert!(
            extensions.contains(&"https://www.cityjson.org/extensions/noise.ext.json".to_string())
        );
    }

    #[test]
    fn test_cityjson_extensions_sorted() {
        // Verify extensions are returned sorted
        let mut temp_file = NamedTempFile::new().unwrap();
        let cityjson = r#"{
            "type": "CityJSON",
            "version": "2.0",
            "extensions": {
                "https://z-extension.org/z.ext.json": "Z",
                "https://a-extension.org/a.ext.json": "A"
            },
            "CityObjects": {},
            "vertices": []
        }"#;

        writeln!(temp_file, "{}", cityjson).unwrap();
        temp_file.flush().unwrap();

        let reader = CityJSONReader::new(temp_file.path()).unwrap();
        let extensions = reader.extensions().unwrap();

        assert_eq!(extensions.len(), 2);
        // Should be sorted alphabetically
        assert_eq!(extensions[0], "https://a-extension.org/a.ext.json");
        assert_eq!(extensions[1], "https://z-extension.org/z.ext.json");
    }
}
