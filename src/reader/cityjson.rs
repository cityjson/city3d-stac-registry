//! CityJSON format reader

use crate::error::{CityJsonStacError, Result};
use crate::metadata::{AttributeDefinition, AttributeType, BBox3D, Transform, CRS};
use crate::reader::CityModelMetadataReader;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

/// Reader for CityJSON format files (.json)
pub struct CityJSONReader {
    file_path: PathBuf,
    // Cached parsed data (lazy loaded)
    data: Option<Value>,
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
            data: None,
        })
    }

    /// Lazy load and cache JSON data
    fn ensure_loaded(&mut self) -> Result<&Value> {
        if self.data.is_none() {
            let file = File::open(&self.file_path)?;
            let reader = BufReader::new(file);
            self.data = Some(serde_json::from_reader(reader)?);
        }
        Ok(self.data.as_ref().unwrap())
    }

    /// Get a reference to the cached data (must call ensure_loaded first)
    fn data(&mut self) -> Result<&Value> {
        self.ensure_loaded()
    }

    /// Extract bbox from metadata or compute from vertices
    fn extract_bbox(&mut self) -> Result<BBox3D> {
        let data = self.data()?;

        // Try to get from metadata.geographicalExtent first
        if let Some(metadata) = data.get("metadata") {
            if let Some(extent) = metadata.get("geographicalExtent") {
                if let Some(arr) = extent.as_array() {
                    if arr.len() == 6 {
                        return Ok(BBox3D::new(
                            arr[0].as_f64().unwrap_or(0.0),
                            arr[1].as_f64().unwrap_or(0.0),
                            arr[2].as_f64().unwrap_or(0.0),
                            arr[3].as_f64().unwrap_or(0.0),
                            arr[4].as_f64().unwrap_or(0.0),
                            arr[5].as_f64().unwrap_or(0.0),
                        ));
                    }
                }
            }
        }

        // Fallback: compute from vertices
        // Extract transform first to avoid borrow issues
        let transform = self.extract_transform()?;
        let data = self.data()?; // Re-borrow after transform extraction

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
                            let mut coords = [
                                v[0].as_f64().unwrap_or(0.0),
                                v[1].as_f64().unwrap_or(0.0),
                                v[2].as_f64().unwrap_or(0.0),
                            ];

                            // Apply transform if present
                            if let Some(ref t) = transform {
                                let compressed = [
                                    v[0].as_i64().unwrap_or(0) as i32,
                                    v[1].as_i64().unwrap_or(0) as i32,
                                    v[2].as_i64().unwrap_or(0) as i32,
                                ];
                                coords = t.apply(&compressed);
                            }

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

    /// Extract CRS from metadata
    fn extract_crs(&mut self) -> Result<CRS> {
        let data = self.data()?;

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

    /// Extract transform from data
    fn extract_transform(&mut self) -> Result<Option<Transform>> {
        let data = self.data()?;

        if let Some(transform) = data.get("transform") {
            let scale = transform
                .get("scale")
                .and_then(|s| s.as_array())
                .and_then(|arr| {
                    if arr.len() == 3 {
                        Some([
                            arr[0].as_f64()?,
                            arr[1].as_f64()?,
                            arr[2].as_f64()?,
                        ])
                    } else {
                        None
                    }
                });

            let translate = transform
                .get("translate")
                .and_then(|t| t.as_array())
                .and_then(|arr| {
                    if arr.len() == 3 {
                        Some([
                            arr[0].as_f64()?,
                            arr[1].as_f64()?,
                            arr[2].as_f64()?,
                        ])
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

    /// Extract LODs from city objects
    fn extract_lods(&mut self) -> Result<Vec<String>> {
        let data = self.data()?;
        let mut lods = HashSet::new();

        if let Some(city_objects) = data.get("CityObjects") {
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

        let mut lods: Vec<String> = lods.into_iter().collect();
        lods.sort();
        Ok(lods)
    }

    /// Extract city object types
    fn extract_city_object_types(&mut self) -> Result<Vec<String>> {
        let data = self.data()?;
        let mut types = HashSet::new();

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

        let mut types: Vec<String> = types.into_iter().collect();
        types.sort();
        Ok(types)
    }

    /// Count city objects
    fn count_city_objects(&mut self) -> Result<usize> {
        let data = self.data()?;

        if let Some(city_objects) = data.get("CityObjects") {
            if let Some(objects) = city_objects.as_object() {
                return Ok(objects.len());
            }
        }

        Ok(0)
    }

    /// Extract attribute schema
    fn extract_attributes(&mut self) -> Result<Vec<AttributeDefinition>> {
        let data = self.data()?;
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
}

impl CityModelMetadataReader for CityJSONReader {
    fn bbox(&self) -> Result<BBox3D> {
        let mut reader = Self {
            file_path: self.file_path.clone(),
            data: self.data.clone(),
        };
        reader.extract_bbox()
    }

    fn crs(&self) -> Result<CRS> {
        let mut reader = Self {
            file_path: self.file_path.clone(),
            data: self.data.clone(),
        };
        reader.extract_crs()
    }

    fn lods(&self) -> Result<Vec<String>> {
        let mut reader = Self {
            file_path: self.file_path.clone(),
            data: self.data.clone(),
        };
        reader.extract_lods()
    }

    fn city_object_types(&self) -> Result<Vec<String>> {
        let mut reader = Self {
            file_path: self.file_path.clone(),
            data: self.data.clone(),
        };
        reader.extract_city_object_types()
    }

    fn city_object_count(&self) -> Result<usize> {
        let mut reader = Self {
            file_path: self.file_path.clone(),
            data: self.data.clone(),
        };
        reader.count_city_objects()
    }

    fn attributes(&self) -> Result<Vec<AttributeDefinition>> {
        let mut reader = Self {
            file_path: self.file_path.clone(),
            data: self.data.clone(),
        };
        reader.extract_attributes()
    }

    fn encoding(&self) -> &'static str {
        "CityJSON"
    }

    fn version(&self) -> Result<String> {
        let mut reader = Self {
            file_path: self.file_path.clone(),
            data: self.data.clone(),
        };
        let data = reader.data()?;

        if let Some(version) = data.get("version") {
            if let Some(v_str) = version.as_str() {
                return Ok(v_str.to_string());
            }
        }

        Ok("1.0".to_string()) // Default version
    }

    fn file_path(&self) -> &Path {
        &self.file_path
    }

    fn transform(&self) -> Result<Option<Transform>> {
        let mut reader = Self {
            file_path: self.file_path.clone(),
            data: self.data.clone(),
        };
        reader.extract_transform()
    }

    fn metadata(&self) -> Result<Option<Value>> {
        let mut reader = Self {
            file_path: self.file_path.clone(),
            data: self.data.clone(),
        };
        let data = reader.data()?;

        Ok(data.get("metadata").cloned())
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
}
