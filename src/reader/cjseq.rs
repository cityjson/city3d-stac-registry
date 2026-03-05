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
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::sync::RwLock;

/// Aggregated metadata computed from all features in the CityJSONSeq file
#[derive(Clone)]
struct AggregatedMetadata {
    /// All unique LODs across all features.
    lods: HashSet<String>,
    /// All unique city object types (excluding extension types starting with '+')
    city_object_types: HashSet<String>,
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
    /// Metadata header from first line (typed CityJSON struct)
    metadata_header: cjseq::CityJSON,
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

        let metadata_header =
            super::parse_cityjson(&first_line).map_err(CityJsonStacError::Other)?;

        // Stream through remaining lines to aggregate statistics
        let mut aggregated = AggregatedMetadata {
            lods: HashSet::new(),
            city_object_types: HashSet::new(),
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

            let feature = cjseq::CityJSONFeature::from_str(&line).map_err(|e| {
                CityJsonStacError::Other(format!("Failed to parse CityJSONFeature: {e}"))
            })?;
            Self::process_feature(&feature, &mut aggregated);
        }

        Ok(Self {
            file_path: file_path.to_path_buf(),
            metadata_header,
            aggregated: RwLock::new(Some(aggregated)),
        })
    }

    /// Create a CityJSONSeq reader from in-memory content
    ///
    /// This is used for remote files that have been downloaded as strings.
    /// The `virtual_path` is used for display purposes (e.g., the original filename).
    pub fn from_content(content: &str, virtual_path: PathBuf) -> Result<Self> {
        let mut lines = content.lines();

        // First line: CityJSON header (metadata only)
        let first_line = lines
            .next()
            .ok_or_else(|| CityJsonStacError::Other("Empty CityJSONSeq content".to_string()))?;

        let metadata_header =
            super::parse_cityjson(first_line).map_err(CityJsonStacError::Other)?;

        // Stream through remaining lines to aggregate statistics
        let mut aggregated = AggregatedMetadata {
            lods: HashSet::new(),
            city_object_types: HashSet::new(),
            city_object_count: 0,
            attributes: HashMap::new(),
            has_semantic_surfaces: false,
            has_textures: false,
            has_materials: false,
        };

        for line in lines {
            if line.trim().is_empty() {
                continue;
            }

            let feature = cjseq::CityJSONFeature::from_str(line).map_err(|e| {
                CityJsonStacError::Other(format!("Failed to parse CityJSONFeature: {e}"))
            })?;
            Self::process_feature(&feature, &mut aggregated);
        }

        Ok(Self {
            file_path: virtual_path,
            metadata_header,
            aggregated: RwLock::new(Some(aggregated)),
        })
    }

    /// Create a CityJSONSeq reader by streaming from a remote URL
    ///
    /// Instead of downloading the entire file into memory, this streams the
    /// data line-by-line using `object_store`. The first line (header) is parsed
    /// to get metadata, then features are processed incrementally.
    ///
    /// This is more memory-efficient for large remote `.jsonl` files since
    /// only one line is held in memory at a time.
    pub async fn from_url_stream(url: &str, virtual_path: PathBuf) -> Result<Self> {
        use futures::TryStreamExt;
        use tokio::io::AsyncBufReadExt;
        use tokio_util::io::StreamReader;

        let parsed_url = url::Url::parse(url).map_err(CityJsonStacError::UrlError)?;
        let (store, path) =
            object_store::parse_url_opts(&parsed_url, Vec::<(String, String)>::new()).map_err(
                |e| CityJsonStacError::StorageError(format!("Failed to create object store: {e}")),
            )?;

        log::info!("Streaming CityJSONSeq from: {}", url);

        let result = store.get(&path).await?;
        let stream = result.into_stream();

        // Convert object_store byte stream to an async line reader
        let async_reader = StreamReader::new(stream.map_err(std::io::Error::other));
        let buf_reader = tokio::io::BufReader::new(async_reader);
        let mut lines = buf_reader.lines();

        // First line: CityJSON header (metadata only)
        let first_line = lines
            .next_line()
            .await
            .map_err(|e| CityJsonStacError::Other(format!("Failed to read stream: {e}")))?
            .ok_or_else(|| CityJsonStacError::Other("Empty CityJSONSeq stream".to_string()))?;

        let metadata_header =
            super::parse_cityjson(&first_line).map_err(CityJsonStacError::Other)?;

        // Stream through remaining lines to aggregate statistics
        let mut aggregated = AggregatedMetadata {
            lods: HashSet::new(),
            city_object_types: HashSet::new(),
            city_object_count: 0,
            attributes: HashMap::new(),
            has_semantic_surfaces: false,
            has_textures: false,
            has_materials: false,
        };

        while let Some(line) = lines
            .next_line()
            .await
            .map_err(|e| CityJsonStacError::Other(format!("Failed to read stream: {e}")))?
        {
            if line.trim().is_empty() {
                continue;
            }

            let feature = cjseq::CityJSONFeature::from_str(&line).map_err(|e| {
                CityJsonStacError::Other(format!("Failed to parse CityJSONFeature: {e}"))
            })?;
            Self::process_feature(&feature, &mut aggregated);
        }

        log::debug!(
            "Streamed {} city objects from {}",
            aggregated.city_object_count,
            url
        );

        Ok(Self {
            file_path: virtual_path,
            metadata_header,
            aggregated: RwLock::new(Some(aggregated)),
        })
    }

    /// Process a single feature and update aggregated statistics
    fn process_feature(feature: &cjseq::CityJSONFeature, aggregated: &mut AggregatedMetadata) {
        for city_object in feature.city_objects.values() {
            // Collect city object types (excluding extension types starting with '+')
            if !city_object.thetype.starts_with('+') {
                aggregated
                    .city_object_types
                    .insert(city_object.thetype.clone());
            }

            // Collect LODs from geometry and check for semantic surfaces
            if let Some(ref geometries) = city_object.geometry {
                for geom in geometries {
                    if let Some(ref lod) = geom.lod {
                        aggregated.lods.insert(lod.clone());
                    }
                    // Check for semantic surfaces
                    if geom.semantics.is_some() {
                        aggregated.has_semantic_surfaces = true;
                    }
                }
            }

            // Collect attributes
            if let Some(ref attrs) = city_object.attributes {
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

        // Check for textures and materials in appearance
        if let Some(ref appearance) = feature.appearance {
            if appearance.textures.is_some() {
                aggregated.has_textures = true;
            }
            if appearance.materials.is_some() {
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
        if let Some(ref metadata) = self.metadata_header.metadata {
            if let Some(ref rs) = metadata.reference_system {
                // cjseq::ReferenceSystem has authority and code fields
                if rs.authority == "EPSG" {
                    if let Ok(code) = rs.code.parse::<u32>() {
                        return CRS::from_epsg(code);
                    }
                }
            }
        }
        CRS::default()
    }

    /// Extract transform from metadata header
    fn extract_transform_from_header(&self) -> Option<Transform> {
        let scale = &self.metadata_header.transform.scale;
        let translate = &self.metadata_header.transform.translate;

        if scale.len() == 3 && translate.len() == 3 {
            // Check if transform is the default identity (no actual transform)
            let is_default = scale[0] == 1.0
                && scale[1] == 1.0
                && scale[2] == 1.0
                && translate[0] == 0.0
                && translate[1] == 0.0
                && translate[2] == 0.0;

            if is_default {
                return None;
            }

            Some(Transform::new(
                [scale[0], scale[1], scale[2]],
                [translate[0], translate[1], translate[2]],
            ))
        } else {
            None
        }
    }

    /// Extract bbox from metadata header
    fn extract_bbox_from_header(&self) -> Result<BBox3D> {
        if let Some(ref metadata) = self.metadata_header.metadata {
            if let Some(extent) = metadata.geographical_extent {
                return Ok(BBox3D::new(
                    extent[0], extent[1], extent[2], extent[3], extent[4], extent[5],
                ));
            }
        }
        // Default bbox if not found
        Ok(BBox3D::new(0.0, 0.0, 0.0, 0.0, 0.0, 0.0))
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
        let mut lods: Vec<String> = aggregated.lods.into_iter().collect();
        lods.sort();
        Ok(lods)
    }

    fn city_object_types(&self) -> Result<Vec<String>> {
        let aggregated = self.get_aggregated()?;
        let mut types: Vec<String> = aggregated.city_object_types.into_iter().collect();
        types.sort();
        Ok(types)
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
        Ok(self.metadata_header.version.clone())
    }

    fn file_path(&self) -> &Path {
        &self.file_path
    }

    fn transform(&self) -> Result<Option<Transform>> {
        Ok(self.extract_transform_from_header())
    }

    fn metadata(&self) -> Result<Option<Value>> {
        match &self.metadata_header.metadata {
            Some(m) => {
                let value = serde_json::to_value(m).map_err(|e| {
                    CityJsonStacError::Other(format!("Failed to serialize metadata: {e}"))
                })?;
                Ok(Some(value))
            }
            None => Ok(None),
        }
    }

    fn extensions(&self) -> Result<Vec<String>> {
        if let Some(ref ext) = self.metadata_header.extensions {
            if let Some(ext_obj) = ext.as_object() {
                let mut extensions: Vec<String> = ext_obj.keys().cloned().collect();
                extensions.sort();
                return Ok(extensions);
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

    fn streamable(&self) -> bool {
        true
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
        let feature = r#"{"type":"CityJSONFeature","id":"b1","CityObjects":{"b1":{"type":"Building","geometry":[{"type":"Solid","lod":"2","boundaries":[[[[0,0,0]]]],"semantics":{"surfaces":[{"type":"WallSurface"}],"values":[[[0]]]}}]}},"vertices":[[0,0,0]]}"#;

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

    #[test]
    fn test_cityjsonseq_from_content() {
        let content = [
            r#"{"type":"CityJSON","version":"2.0","transform":{"scale":[0.01,0.01,0.01],"translate":[100000,200000,0]},"CityObjects":{},"vertices":[],"metadata":{"geographicalExtent":[1.0,2.0,0.0,10.0,20.0,30.0],"referenceSystem":"https://www.opengis.net/def/crs/EPSG/0/7415"}}"#,
            r#"{"type":"CityJSONFeature","id":"building1","CityObjects":{"building1":{"type":"Building","geometry":[{"type":"Solid","lod":"2","boundaries":[[[[0,0,0]]]]}],"attributes":{"yearOfConstruction":2020}}},"vertices":[[1000,2000,3000]]}"#,
            r#"{"type":"CityJSONFeature","id":"building2","CityObjects":{"building2":{"type":"Building","geometry":[{"type":"Solid","lod":"2.2","boundaries":[[[[0,0,0]]]]}]}},"vertices":[[2000,3000,4000]]}"#,
        ].join("\n");

        let reader =
            CityJSONSeqReader::from_content(&content, PathBuf::from("remote.city.jsonl")).unwrap();

        assert_eq!(reader.version().unwrap(), "2.0");
        assert_eq!(reader.city_object_count().unwrap(), 2);
        assert_eq!(reader.city_object_types().unwrap(), vec!["Building"]);
        assert_eq!(reader.crs().unwrap().epsg, Some(7415));
        assert_eq!(reader.file_path(), Path::new("remote.city.jsonl"));
        assert_eq!(reader.encoding(), "CityJSONSeq");

        let lods = reader.lods().unwrap();
        assert!(lods.contains(&"2".to_string()));
        assert!(lods.contains(&"2.2".to_string()));
    }

    #[test]
    fn test_cityjsonseq_from_content_empty() {
        let result = CityJSONSeqReader::from_content("", PathBuf::from("empty.jsonl"));
        assert!(result.is_err());
    }

    #[test]
    fn test_cityjsonseq_from_content_invalid_header() {
        let result = CityJSONSeqReader::from_content("not valid json", PathBuf::from("bad.jsonl"));
        assert!(result.is_err());
    }

    #[test]
    fn test_cityjsonseq_streamable() {
        let temp_file = create_test_cityjsonseq();
        let reader = CityJSONSeqReader::new(temp_file.path()).unwrap();
        assert!(reader.streamable());
    }

    #[test]
    fn test_cityjsonseq_from_content_streamable() {
        let content = [
            r#"{"type":"CityJSON","version":"2.0","transform":{"scale":[1.0,1.0,1.0],"translate":[0,0,0]},"CityObjects":{},"vertices":[],"metadata":{"geographicalExtent":[0,0,0,1,1,1]}}"#,
            r#"{"type":"CityJSONFeature","id":"b1","CityObjects":{"b1":{"type":"Building","geometry":[]}},"vertices":[]}"#,
        ].join("\n");

        let reader =
            CityJSONSeqReader::from_content(&content, PathBuf::from("test.jsonl")).unwrap();
        assert!(reader.streamable());
    }
}
