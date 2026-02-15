//! CityGML format reader (versions 2.0 and 3.0)
//!
//! Uses streaming XML parser to handle large files efficiently.

use crate::error::{CityJsonStacError, Result};
use crate::metadata::{AttributeDefinition, AttributeType, BBox3D, CRS};
use crate::reader::CityModelMetadataReader;
use quick_xml::events::Event;
use serde_json::Value;
use std::collections::BTreeSet;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

/// CityGML version detected in file
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CityGMLVersion {
    V2_0,
    V3_0,
}

/// Cached metadata extracted from CityGML file
#[derive(Debug)]
struct CityGMLMetadata {
    version: CityGMLVersion,
    bbox: Option<BBox3D>,
    crs: Option<CRS>,
    city_object_count: usize,
    city_object_types: BTreeSet<String>,
    lods: BTreeSet<String>,
    attributes: Vec<AttributeDefinition>,
    has_textures: bool,
    has_materials: bool,
    has_semantic_surfaces: bool,
}

/// Reader for CityGML format files (.gml, .xml)
///
/// Uses streaming XML parser for memory efficiency with large files.
/// Supports both CityGML 2.0 and 3.0 specifications.
pub struct CityGMLReader {
    file_path: PathBuf,
    metadata: RwLock<Option<CityGMLMetadata>>,
}

impl CityGMLReader {
    /// Create a new CityGML reader
    pub fn new(file_path: &Path) -> Result<Self> {
        if !file_path.exists() {
            return Err(CityJsonStacError::IoError(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("File not found: {}", file_path.display()),
            )));
        }

        Ok(Self {
            file_path: file_path.to_path_buf(),
            metadata: RwLock::new(None),
        })
    }

    /// Lazy load and cache metadata using interior mutability
    fn ensure_loaded(&self) -> Result<()> {
        // First check if already loaded with a read lock (cheaper)
        {
            let metadata = self
                .metadata
                .read()
                .map_err(|_| CityJsonStacError::Other("Failed to acquire read lock".to_string()))?;
            if metadata.is_some() {
                return Ok(());
            }
        }

        // Not loaded, acquire write lock and load
        let mut metadata = self
            .metadata
            .write()
            .map_err(|_| CityJsonStacError::Other("Failed to acquire write lock".to_string()))?;

        // Double-check after acquiring write lock
        if metadata.is_none() {
            *metadata = Some(self.parse_metadata()?);
        }
        Ok(())
    }

    /// Parse CityGML metadata using streaming XML parser
    fn parse_metadata(&self) -> Result<CityGMLMetadata> {
        let file = File::open(&self.file_path)?;
        let reader = BufReader::new(file);

        let mut parser = quick_xml::Reader::from_reader(reader);
        parser.config_mut().trim_text(true);

        let mut buf = Vec::new();
        let mut metadata = CityGMLMetadata {
            version: CityGMLVersion::V2_0, // Default, will detect
            bbox: None,
            crs: None,
            city_object_count: 0,
            city_object_types: BTreeSet::new(),
            lods: BTreeSet::new(),
            attributes: Vec::new(),
            has_textures: false,
            has_materials: false,
            has_semantic_surfaces: false,
        };

        let mut attribute_map: BTreeSet<String> = BTreeSet::new();
        let mut depth = 0;
        let mut in_lower_corner = false;
        let mut in_upper_corner = false;
        let mut lower_corner: Option<[f64; 3]> = None;
        let mut upper_corner: Option<[f64; 3]> = None;
        let mut found_root_bbox = false;
        let mut root_srs_name: Option<String> = None;
        let mut in_generic_attribute = false;
        let mut in_attribute_name = false;

        loop {
            match parser.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) => {
                    let name = e.name().into_inner(); // Fix lifetime issue
                    depth += 1;

                    // Detect CityGML version from root element namespace
                    if name == b"CityModel" {
                        for attr in e.attributes().flatten() {
                            let key = attr.key.as_ref();
                            if key == b"xmlns" || key.starts_with(b"xmlns:") {
                                let value = std::str::from_utf8(&attr.value).unwrap_or("");
                                if value.contains("/citygml/3.0") {
                                    metadata.version = CityGMLVersion::V3_0;
                                } else if value.contains("/citygml/2.0") {
                                    metadata.version = CityGMLVersion::V2_0;
                                }
                            }
                        }
                    }

                    // Look for srsName attribute in Envelope element for CRS
                    if let Ok(local_name) = std::str::from_utf8(name) {
                        if let Some(suffix) = local_name.split(':').last() {
                            if suffix == "Envelope" && depth == 3 {
                                for attr in e.attributes().flatten() {
                                    if attr.key.as_ref() == b"srsName" {
                                        if let Ok(srs_name) = std::str::from_utf8(&attr.value) {
                                            root_srs_name = Some(srs_name.to_string());
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Detect city object members (count and type)
                    if name == b"cityObjectMember" {
                        metadata.city_object_count += 1;
                    }

                    // Detect object types (Building, Road, etc.)
                    if let Ok(local_name) = std::str::from_utf8(name) {
                        if let Some(type_name) = local_name.split(':').last() {
                            // Common CityGML object types
                            match type_name {
                                "Building" | "Road" | "Railway" | "TransportationComplex"
                                | "Tunnel" | "Bridge" | "WaterBody" | "PlantCover"
                                | "SolitaryVegetationObject" | "LandUse"
                                | "CityFurniture" | "GenericCityObject" => {
                                    if depth == 3 {
                                        // Direct child of cityObjectMember
                                        let full_type = type_name.to_string();
                                        metadata.city_object_types.insert(full_type);
                                    }
                                }
                                _ => {}
                            }
                        }
                    }

                    // Detect LODs from geometry elements
                    if let Ok(local_name) = std::str::from_utf8(name) {
                        if local_name.contains("lod") && local_name != "lod" {
                            // Extract LOD number from element names like "lod1Solid", "lod2MultiSurface"
                            let lod_part = local_name
                                .split("lod")
                                .nth(1)
                                .and_then(|s| s.chars().next())
                                .map(|c| c.to_string());

                            if let Some(lod) = lod_part {
                                metadata.lods.insert(lod);
                            }
                        }
                    }

                    // Parse bounding box elements at root level (depth 4)
                    if !found_root_bbox {
                        if let Ok(local_name) = std::str::from_utf8(name) {
                            if let Some(suffix) = local_name.split(':').last() {
                                if suffix == "lowerCorner" && depth == 4 {
                                    in_lower_corner = true;
                                } else if suffix == "upperCorner" && depth == 4 {
                                    in_upper_corner = true;
                                }
                            }
                        }
                    }

                    // Detect attributes (CityGML 2.0 format)
                    // <gen:stringAttribute name="..."><gen:value>...</gen:value></gen:stringAttribute>
                    if metadata.version == CityGMLVersion::V2_0 {
                        if let Ok(local_name) = std::str::from_utf8(name) {
                            if let Some(attr_type) = local_name.split(':').last() {
                                if attr_type.ends_with("Attribute")
                                    && attr_type != "genericAttribute" {
                                    // Get the name attribute
                                    for attr in e.attributes().flatten() {
                                        if attr.key.as_ref() == b"name" {
                                            if let Ok(name_str) = std::str::from_utf8(&attr.value) {
                                                attribute_map.insert(name_str.to_string());
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Detect attributes (CityGML 3.0 format)
                    // <genericAttribute><gen:StringAttribute><gen:name>...</gen:name><gen:value>...</gen:value></gen:StringAttribute></genericAttribute>
                    if metadata.version == CityGMLVersion::V3_0 {
                        if let Ok(local_name) = std::str::from_utf8(name) {
                            if let Some(type_name) = local_name.split(':').last() {
                                if type_name == "genericAttribute" {
                                    in_generic_attribute = true;
                                } else if type_name == "name" && in_generic_attribute {
                                    in_attribute_name = true;
                                }
                            }
                        }
                    }
                }

                Ok(Event::Text(ref e)) => {
                    // Parse text content for element values
                    if let Ok(text) = e.unescape() {
                        let text_str = text.trim();

                        // Parse attribute name for CityGML 3.0
                        if in_attribute_name && !text_str.is_empty() {
                            attribute_map.insert(text_str.to_string());
                        }

                        // Parse bounding box coordinates
                        if in_lower_corner {
                            let coords: Vec<f64> = text_str
                                .split_whitespace()
                                .filter_map(|s| s.parse::<f64>().ok())
                                .collect();
                            if coords.len() >= 3 {
                                lower_corner = Some([coords[0], coords[1], coords[2]]);
                            }
                        } else if in_upper_corner {
                            let coords: Vec<f64> = text_str
                                .split_whitespace()
                                .filter_map(|s| s.parse::<f64>().ok())
                                .collect();
                            if coords.len() >= 3 {
                                upper_corner = Some([coords[0], coords[1], coords[2]]);
                            }
                        }
                    }
                }

                Ok(Event::End(ref e)) => {
                    let name = e.name().into_inner(); // Fix lifetime issue
                    depth -= 1;

                    if let Ok(local_name) = std::str::from_utf8(name) {
                        if let Some(suffix) = local_name.split(':').last() {
                            if suffix == "lowerCorner" {
                                in_lower_corner = false;
                            } else if suffix == "upperCorner" {
                                in_upper_corner = false;
                            }

                            // Reset CityGML 3.0 attribute parsing state
                            if metadata.version == CityGMLVersion::V3_0 {
                                if suffix == "genericAttribute" {
                                    in_generic_attribute = false;
                                } else if suffix == "name" {
                                    in_attribute_name = false;
                                }
                            }
                        }
                    }

                    // Check if we have both corners for root-level bbox
                    if !found_root_bbox {
                        if let (Some(lower), Some(upper)) = (lower_corner, upper_corner) {
                            metadata.bbox = Some(BBox3D::new(
                                lower[0], lower[1], lower[2],
                                upper[0], upper[1], upper[2],
                            ));
                            found_root_bbox = true;
                        }
                    }

                    if depth == 0 {
                        break; // End of root element
                    }
                }

                Ok(Event::Eof) => break,

                Err(e) => {
                    return Err(CityJsonStacError::Other(format!("XML parsing error: {e}")))
                }

                _ => {}
            }

            buf.clear();
        }

        // Parse CRS from srsName
        if let Some(srs_name) = root_srs_name {
            metadata.crs = CRS::from_cityjson_url(&srs_name);
        }

        // Convert attribute map to sorted vec
        metadata.attributes = attribute_map
            .into_iter()
            .map(|name| AttributeDefinition::new(name, AttributeType::String))
            .collect();

        Ok(metadata)
    }
}

impl CityModelMetadataReader for CityGMLReader {
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
        metadata
            .as_ref()
            .and_then(|m| m.crs.clone())
            .ok_or_else(|| CityJsonStacError::MetadataError("CRS not found".to_string()))
    }

    fn encoding(&self) -> &'static str {
        "CityGML"
    }

    fn version(&self) -> Result<String> {
        self.ensure_loaded()?;
        let metadata = self
            .metadata
            .read()
            .map_err(|_| CityJsonStacError::Other("Failed to acquire read lock".to_string()))?;
        Ok(match metadata.as_ref().unwrap().version {
            CityGMLVersion::V2_0 => "2.0",
            CityGMLVersion::V3_0 => "3.0",
        }
        .to_string())
    }

    fn city_object_count(&self) -> Result<usize> {
        self.ensure_loaded()?;
        let metadata = self
            .metadata
            .read()
            .map_err(|_| CityJsonStacError::Other("Failed to acquire read lock".to_string()))?;
        Ok(metadata.as_ref().unwrap().city_object_count)
    }

    fn city_object_types(&self) -> Result<Vec<String>> {
        self.ensure_loaded()?;
        let metadata = self
            .metadata
            .read()
            .map_err(|_| CityJsonStacError::Other("Failed to acquire read lock".to_string()))?;
        Ok(metadata.as_ref().unwrap().city_object_types.iter().cloned().collect())
    }

    fn lods(&self) -> Result<Vec<String>> {
        self.ensure_loaded()?;
        let metadata = self
            .metadata
            .read()
            .map_err(|_| CityJsonStacError::Other("Failed to acquire read lock".to_string()))?;
        Ok(metadata.as_ref().unwrap().lods.iter().cloned().collect())
    }

    fn attributes(&self) -> Result<Vec<AttributeDefinition>> {
        self.ensure_loaded()?;
        let metadata = self
            .metadata
            .read()
            .map_err(|_| CityJsonStacError::Other("Failed to acquire read lock".to_string()))?;
        Ok(metadata.as_ref().unwrap().attributes.clone())
    }

    fn file_path(&self) -> &Path {
        &self.file_path
    }

    fn transform(&self) -> Result<Option<crate::metadata::Transform>> {
        Ok(None) // CityGML doesn't use vertex compression
    }

    fn metadata(&self) -> Result<Option<Value>> {
        Ok(None) // CityGML has XML, not JSON metadata
    }

    fn extensions(&self) -> Result<Vec<String>> {
        // TODO: Implement ADE detection from namespace declarations
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
