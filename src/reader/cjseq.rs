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

// FIXME: don't use Value, use cjseq::CityJSON instead
use serde_json::Value;
use std::collections::{BTreeSet, HashMap};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::sync::RwLock;

/// Aggregated metadata computed from all features in the CityJSONSeq file
#[derive(Clone)]
struct AggregatedMetadata {
    /// All unique LODs across all features.
    // FIXME: we don't need to use BTreeSet here, just use HashSet
    lods: BTreeSet<String>,
    /// All unique city object types (excluding extension types starting with '+')
    // FIXME: Use strict type exposed from cjseq lib. Also we don't need to use BTreeSet here, just use HashSet
    city_object_types: BTreeSet<String>,
    /// Total count of city objects across all features
    city_object_count: usize,
    /// All attributes found across features
    attributes: HashMap<String, AttributeType>,
    /// Whether any feature has semantic surfaces
    has_semantic_surfaces: bool,
    /// Whether any feature has textures
    has_textures: bool,
    /// Whether any feature has materials
    has_materials: bool,
}

/// Reader for CityJSON Text Sequences format files (.city.jsonl, .jsonl, .cjseq)
///
/// Uses streaming approach: reads the first line as metadata header,
/// then streams through remaining features to aggregate statistics.
pub struct CityJSONSeqReader {
    file_path: PathBuf,
    /// Metadata header from first line (as raw JSON Value for compatibility)
    metadata_header: Value,
    /// Aggregated statistics (computed during construction)
    aggregated: RwLock<Option<AggregatedMetadata>>,
}

impl CityJSONSeqReader {
    /// Create a new CityJSONSeq reader
    ///
    /// This reads the first line as the metadata header and then streams
    /// through all features to aggregate statistics.
    pub fn new(file_path: &Path) -> Result<Self> {
        if !file_path.exists() {
            return Err(CityJsonStacError::IoError(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("File not found: {}", file_path.display()),
            )));
        }

        let file = File::open(file_path)?;
        let reader = BufReader::new(file);
        let mut lines = reader.lines();

        // First line: CityJSON header (metadata only)
        let first_line = lines
            .next()
            .ok_or_else(|| CityJsonStacError::Other("Empty CityJSONSeq file".to_string()))??;

        let metadata_header: Value = serde_json::from_str(&first_line).map_err(|e| {
            CityJsonStacError::Other(format!("Failed to parse CityJSONSeq header: {e}"))
        })?;

        // Stream through remaining lines to aggregate statistics
        let mut aggregated = AggregatedMetadata {
            lods: BTreeSet::new(),
            city_object_types: BTreeSet::new(),
            city_object_count: 0,
            attributes: HashMap::new(),
            has_semantic_surfaces: false,
            has_textures: false,
            has_materials: false,
        };

        // Process each feature line
        for line_result in lines {
            let line = line_result?;
            if line.trim().is_empty() {
                continue; // Skip empty lines
            }

            match serde_json::from_str::<Value>(&line) {
                Ok(feature) => {
                    Self::process_feature(&feature, &mut aggregated);
                }
                Err(e) => {
                    return Err(CityJsonStacError::Other(format!(
                        "Failed to parse CityJSONFeature: {e}"
                    )));
                }
            }
        }

        Ok(Self {
            file_path: file_path.to_path_buf(),
            metadata_header,
            aggregated: RwLock::new(Some(aggregated)),
        })
    }

    /// Process a single feature and update aggregated statistics
    fn process_feature(feature: &Value, aggregated: &mut AggregatedMetadata) {
        // Get CityObjects from the feature
        if let Some(city_objects) = feature
            .get("CityObjects")
            .or_else(|| feature.get("city_objects"))
        {
            if let Some(objs) = city_objects.as_object() {
                for (_id, city_object) in objs {
                    // Collect city object types (excluding extension types starting with '+')
                    if let Some(type_val) = city_object
                        .get("type")
                        .or_else(|| city_object.get("thetype"))
                    {
                        if let Some(type_str) = type_val.as_str() {
                            if !type_str.starts_with('+') {
                                aggregated.city_object_types.insert(type_str.to_string());
                            }
                        }
                    }

                    // Collect LODs from geometry
                    if let Some(geometries) = city_object.get("geometry") {
                        if let Some(geom_array) = geometries.as_array() {
                            for geom in geom_array {
                                if let Some(lod) = geom.get("lod").and_then(|v| v.as_str()) {
                                    aggregated.lods.insert(lod.to_string());
                                }
                                // Check for semantic surfaces
                                if geom.get("semantics").is_some() {
                                    aggregated.has_semantic_surfaces = true;
                                }
                            }
                        }
                    }

                    // Collect attributes
                    if let Some(attrs) = city_object.get("attributes") {
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
                                aggregated
                                    .attributes
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

                    // Increment count
                    aggregated.city_object_count += 1;
                }
            }
        }

        // Check for textures and materials in appearance
        if let Some(appearance) = feature
            .get("appearance")
            .or_else(|| feature.get("Appearance"))
        {
            if appearance
                .get("textures")
                .or_else(|| appearance.get("Textures"))
                .is_some()
            {
                aggregated.has_textures = true;
            }
            if appearance
                .get("materials")
                .or_else(|| appearance.get("Materials"))
                .is_some()
            {
                aggregated.has_materials = true;
            }
        }
    }

    /// Get the aggregated metadata (lazy loaded)
    fn get_aggregated(&self) -> Result<AggregatedMetadata> {
        // First check with a read lock
        {
            let aggregated = self
                .aggregated
                .read()
                .map_err(|_| CityJsonStacError::Other("Failed to acquire read lock".to_string()))?;
            if let Some(ref agg) = *aggregated {
                return Ok(agg.clone());
            }
        }

        // This shouldn't happen since we populate it in new(), but handle gracefully
        Err(CityJsonStacError::Other(
            "Aggregated metadata not initialized".to_string(),
        ))
    }

    /// Extract CRS from metadata header
    fn extract_crs_from_header(&self) -> CRS {
        if let Some(metadata) = self.metadata_header.get("metadata") {
            // Try referenceSystem as string URL (common in CityJSONSeq)
            if let Some(rs) = metadata.get("referenceSystem") {
                if let Some(rs_str) = rs.as_str() {
                    // Handle URL format: "https://www.opengis.net/def/crs/EPSG/0/7415"
                    if rs_str.contains("EPSG") {
                        let parts: Vec<&str> = rs_str.split('/').collect();
                        if let Some(last) = parts.last() {
                            if let Ok(code) = last.parse::<u32>() {
                                return CRS::from_epsg(code);
                            }
                        }
                    }
                }
                // Try referenceSystem as object
                if let Some(rs_obj) = rs.as_object() {
                    // Try base_url field
                    if let Some(base_url) = rs_obj.get("base_url").and_then(|v| v.as_str()) {
                        if base_url.contains("EPSG") {
                            let parts: Vec<&str> = base_url.split('/').collect();
                            if let Some(last) = parts.last() {
                                if let Ok(code) = last.parse::<u32>() {
                                    return CRS::from_epsg(code);
                                }
                            }
                        }
                    }
                    // Try code field
                    if let Some(code_val) = rs_obj.get("code") {
                        if let Some(code_str) = code_val.as_str() {
                            if let Ok(code) = code_str.parse::<u32>() {
                                // Check authority
                                if rs_obj.get("authority").and_then(|v| v.as_str()) == Some("EPSG")
                                {
                                    return CRS::from_epsg(code);
                                }
                            }
                        }
                    }
                }
            }

            // Fallback to CRS string (CityJSON 1.0)
            if let Some(crs_value) = metadata.get("CRS") {
                if let Some(crs_str) = crs_value.as_str() {
                    if let Some(code_str) = crs_str.strip_prefix("EPSG:") {
                        if let Ok(code) = code_str.parse::<u32>() {
                            return CRS::from_epsg(code);
                        }
                    }
                }
            }
        }
        CRS::default()
    }

    /// Extract transform from metadata header
    fn extract_transform_from_header(&self) -> Option<Transform> {
        if let Some(transform_obj) = self.metadata_header.get("transform") {
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
                    return Some(Transform::new(s, t));
                }
            }
        }
        None
    }

    /// Extract bbox from metadata header
    fn extract_bbox_from_header(&self) -> Result<BBox3D> {
        if let Some(metadata) = self.metadata_header.get("metadata") {
            // Try geographicalExtent first
            if let Some(extent) = metadata
                .get("geographicalExtent")
                .or_else(|| metadata.get("geographicExtent"))
            {
                if let Some(arr) = extent.as_array() {
                    let vals: Vec<f64> = arr.iter().filter_map(|v| v.as_f64()).collect();
                    if vals.len() == 6 {
                        return Ok(BBox3D::new(
                            vals[0], vals[1], vals[2], vals[3], vals[4], vals[5],
                        ));
                    }
                }
            }
        }
        // Default bbox if not found
        Ok(BBox3D::new(0.0, 0.0, 0.0, 0.0, 0.0, 0.0))
    }

    /// Extract version from metadata header
    fn extract_version_from_header(&self) -> String {
        self.metadata_header
            .get("version")
            .and_then(|v| v.as_str())
            .unwrap_or("1.0")
            .to_string()
    }
}

impl CityModelMetadataReader for CityJSONSeqReader {
    fn bbox(&self) -> Result<BBox3D> {
        self.extract_bbox_from_header()
    }

    fn crs(&self) -> Result<CRS> {
        Ok(self.extract_crs_from_header())
    }

    fn lods(&self) -> Result<Vec<String>> {
        let aggregated = self.get_aggregated()?;
        Ok(aggregated.lods.into_iter().collect())
    }

    fn city_object_types(&self) -> Result<Vec<String>> {
        let aggregated = self.get_aggregated()?;
        Ok(aggregated.city_object_types.into_iter().collect())
    }

    fn city_object_count(&self) -> Result<usize> {
        let aggregated = self.get_aggregated()?;
        Ok(aggregated.city_object_count)
    }

    fn attributes(&self) -> Result<Vec<AttributeDefinition>> {
        let aggregated = self.get_aggregated()?;
        let mut attributes: Vec<_> = aggregated
            .attributes
            .into_iter()
            .map(|(name, attr_type)| AttributeDefinition::new(&name, attr_type))
            .collect();
        attributes.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(attributes)
    }

    fn encoding(&self) -> &'static str {
        "CityJSONSeq"
    }

    fn version(&self) -> Result<String> {
        Ok(self.extract_version_from_header())
    }

    fn file_path(&self) -> &Path {
        &self.file_path
    }

    fn transform(&self) -> Result<Option<Transform>> {
        Ok(self.extract_transform_from_header())
    }

    fn metadata(&self) -> Result<Option<Value>> {
        Ok(self.metadata_header.get("metadata").cloned())
    }

    fn extensions(&self) -> Result<Vec<String>> {
        // Extensions are in the metadata header
        if let Some(metadata) = self.metadata_header.get("metadata") {
            if let Some(ext) = metadata.get("extensions") {
                if let Some(ext_obj) = ext.as_object() {
                    let mut extensions: Vec<String> = ext_obj.keys().cloned().collect();
                    extensions.sort();
                    return Ok(extensions);
                }
            }
        }
        Ok(Vec::new())
    }

    fn semantic_surfaces(&self) -> Result<bool> {
        let aggregated = self.get_aggregated()?;
        Ok(aggregated.has_semantic_surfaces)
    }

    fn textures(&self) -> Result<bool> {
        let aggregated = self.get_aggregated()?;
        Ok(aggregated.has_textures)
    }

    fn materials(&self) -> Result<bool> {
        let aggregated = self.get_aggregated()?;
        Ok(aggregated.has_materials)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn create_test_cityjsonseq() -> NamedTempFile {
        let mut temp_file = NamedTempFile::new().unwrap();

        // Header line (CityJSON with metadata but no CityObjects)
        let header = r#"{"type":"CityJSON","version":"2.0","transform":{"scale":[0.01,0.01,0.01],"translate":[100000,200000,0]},"CityObjects":{},"vertices":[],"metadata":{"geographicalExtent":[1.0,2.0,0.0,10.0,20.0,30.0],"referenceSystem":"https://www.opengis.net/def/crs/EPSG/0/7415"}}"#;

        // Feature line 1
        let feature1 = r#"{"type":"CityJSONFeature","id":"building1","CityObjects":{"building1":{"type":"Building","geometry":[{"type":"Solid","lod":"2","boundaries":[[[[0,0,0]]]]}],"attributes":{"yearOfConstruction":2020,"function":"residential"}}},"vertices":[[1000,2000,3000]]}"#;

        // Feature line 2
        let feature2 = r#"{"type":"CityJSONFeature","id":"building2","CityObjects":{"building2":{"type":"Building","geometry":[{"type":"Solid","lod":"2.2","boundaries":[[[[0,0,0]]]]}],"attributes":{"yearOfConstruction":2021}}},"vertices":[[2000,3000,4000]]}"#;

        writeln!(temp_file, "{}", header).unwrap();
        writeln!(temp_file, "{}", feature1).unwrap();
        writeln!(temp_file, "{}", feature2).unwrap();
        temp_file.flush().unwrap();
        temp_file
    }

    #[test]
    fn test_cityjsonseq_reader_creation() {
        let temp_file = create_test_cityjsonseq();
        let reader = CityJSONSeqReader::new(temp_file.path());
        assert!(reader.is_ok());
    }

    #[test]
    fn test_cityjsonseq_reader_not_found() {
        let reader = CityJSONSeqReader::new(Path::new("/nonexistent/file.jsonl"));
        assert!(reader.is_err());
    }

    #[test]
    fn test_cityjsonseq_extract_version() {
        let temp_file = create_test_cityjsonseq();
        let reader = CityJSONSeqReader::new(temp_file.path()).unwrap();
        let version = reader.version().unwrap();
        assert_eq!(version, "2.0");
    }

    #[test]
    fn test_cityjsonseq_extract_city_objects_count() {
        let temp_file = create_test_cityjsonseq();
        let reader = CityJSONSeqReader::new(temp_file.path()).unwrap();
        let count = reader.city_object_count().unwrap();
        assert_eq!(count, 2); // 2 features, each with 1 city object
    }

    #[test]
    fn test_cityjsonseq_extract_types() {
        let temp_file = create_test_cityjsonseq();
        let reader = CityJSONSeqReader::new(temp_file.path()).unwrap();
        let types = reader.city_object_types().unwrap();
        assert_eq!(types, vec!["Building"]);
    }

    #[test]
    fn test_cityjsonseq_extract_lods() {
        let temp_file = create_test_cityjsonseq();
        let reader = CityJSONSeqReader::new(temp_file.path()).unwrap();
        let lods = reader.lods().unwrap();
        assert!(lods.contains(&"2".to_string()));
        assert!(lods.contains(&"2.2".to_string()));
    }

    #[test]
    fn test_cityjsonseq_extract_bbox() {
        let temp_file = create_test_cityjsonseq();
        let reader = CityJSONSeqReader::new(temp_file.path()).unwrap();
        let bbox = reader.bbox().unwrap();
        assert_eq!(bbox.xmin, 1.0);
        assert_eq!(bbox.ymin, 2.0);
        assert_eq!(bbox.xmax, 10.0);
        assert_eq!(bbox.ymax, 20.0);
    }

    #[test]
    fn test_cityjsonseq_extract_crs() {
        let temp_file = create_test_cityjsonseq();
        let reader = CityJSONSeqReader::new(temp_file.path()).unwrap();
        let crs = reader.crs().unwrap();
        assert_eq!(crs.epsg, Some(7415));
    }

    #[test]
    fn test_cityjsonseq_extract_attributes() {
        let temp_file = create_test_cityjsonseq();
        let reader = CityJSONSeqReader::new(temp_file.path()).unwrap();
        let attrs = reader.attributes().unwrap();

        let attr_names: Vec<&str> = attrs.iter().map(|a| a.name.as_str()).collect();
        assert!(attr_names.contains(&"yearOfConstruction"));
        assert!(attr_names.contains(&"function"));
    }

    #[test]
    fn test_cityjsonseq_encoding() {
        let temp_file = create_test_cityjsonseq();
        let reader = CityJSONSeqReader::new(temp_file.path()).unwrap();
        assert_eq!(reader.encoding(), "CityJSONSeq");
    }

    #[test]
    fn test_cityjsonseq_transform() {
        let temp_file = create_test_cityjsonseq();
        let reader = CityJSONSeqReader::new(temp_file.path()).unwrap();
        let transform = reader.transform().unwrap();
        assert!(transform.is_some());

        let t = transform.unwrap();
        assert_eq!(t.scale, [0.01, 0.01, 0.01]);
        assert_eq!(t.translate, [100000.0, 200000.0, 0.0]);
    }

    #[test]
    fn test_cityjsonseq_metadata() {
        let temp_file = create_test_cityjsonseq();
        let reader = CityJSONSeqReader::new(temp_file.path()).unwrap();
        let metadata = reader.metadata().unwrap();
        assert!(metadata.is_some());
    }

    #[test]
    fn test_cityjsonseq_semantic_surfaces() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let header = r#"{"type":"CityJSON","version":"2.0","transform":{"scale":[1.0,1.0,1.0],"translate":[0,0,0]},"CityObjects":{},"vertices":[],"metadata":{"geographicalExtent":[0,0,0,1,1,1]}}"#;
        let feature = r#"{"type":"CityJSONFeature","id":"b1","CityObjects":{"b1":{"type":"Building","geometry":[{"type":"Solid","lod":"2","boundaries":[[[[0,0,0]]]],"semantics":{"surfaces":[{"type":"Wall"}]}}]}},"vertices":[[0,0,0]]}"#;

        writeln!(temp_file, "{}", header).unwrap();
        writeln!(temp_file, "{}", feature).unwrap();
        temp_file.flush().unwrap();

        let reader = CityJSONSeqReader::new(temp_file.path()).unwrap();
        assert!(reader.semantic_surfaces().unwrap());
    }

    #[test]
    fn test_cityjsonseq_no_semantic_surfaces() {
        let temp_file = create_test_cityjsonseq();
        let reader = CityJSONSeqReader::new(temp_file.path()).unwrap();
        assert!(!reader.semantic_surfaces().unwrap());
    }
}
