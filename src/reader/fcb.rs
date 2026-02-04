//! FlatCityBuf format reader (.fcb)
//!
//! Reads the binary FlatCityBuf format which is an efficient serialization
//! of CityJSON using FlatBuffers.

use crate::error::{CityJsonStacError, Result};
use crate::metadata::{AttributeDefinition, AttributeType, BBox3D, Transform, CRS};
use crate::reader::CityModelMetadataReader;
use fcb_core::fb::feature_generated::CityObjectType;
use fcb_core::fb::header_generated::ColumnType;
use fcb_core::FcbReader;
use std::collections::BTreeSet;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

/// Reader for FlatCityBuf format files (.fcb)
///
/// Uses `RwLock` for interior mutability to enable lazy loading
/// while maintaining thread-safety (`Send + Sync` bounds).
///
/// The FlatCityBuf format stores CityJSON data in a binary format using FlatBuffers,
/// which allows for efficient random access and streaming reads.
pub struct FlatCityBufReader {
    file_path: PathBuf,
    /// Cached FcbReader (lazy loaded via interior mutability)
    /// We cache the header data rather than the reader itself because
    /// FcbReader holds a reference to the underlying file
    cached_header_data: RwLock<Option<CachedHeaderData>>,
    /// Cached data from streaming through features (lazy loaded separately)
    cached_streamed_data: RwLock<Option<CachedStreamedData>>,
}

/// Cached data extracted by streaming through FCB features
/// This is loaded separately from header data since it requires full file traversal
struct CachedStreamedData {
    lods: Vec<String>,
    city_object_types: Vec<String>,
    /// Total count of CityObjects across all features
    /// Note: A single FCB feature can contain multiple CityObjects
    city_object_count: usize,
}

/// Cached data extracted from the FCB header
/// This avoids keeping a reference to the file's lifetime
struct CachedHeaderData {
    version: String,
    geographical_extent: Option<(f64, f64, f64, f64, f64, f64)>,
    reference_system_code: Option<i32>,
    transform: Option<(f64, f64, f64, f64, f64, f64)>, // scale_x, scale_y, scale_z, translate_x, translate_y, translate_z
    columns: Vec<FcbColumn>,
    metadata_json: Option<serde_json::Value>,
    extensions: Vec<String>,
}

/// Simplified column representation extracted from FCB header
struct FcbColumn {
    name: String,
    column_type: u8,
    nullable: bool,
    description: Option<String>,
}

impl FlatCityBufReader {
    /// Create a new FlatCityBuf reader
    pub fn new(file_path: &Path) -> Result<Self> {
        if !file_path.exists() {
            return Err(CityJsonStacError::IoError(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("File not found: {}", file_path.display()),
            )));
        }

        Ok(Self {
            file_path: file_path.to_path_buf(),
            cached_header_data: RwLock::new(None),
            cached_streamed_data: RwLock::new(None),
        })
    }

    /// Lazy load and cache header data using interior mutability
    fn ensure_loaded(&self) -> Result<()> {
        // First check if already loaded with a read lock (cheaper)
        {
            let data = self
                .cached_header_data
                .read()
                .map_err(|_| CityJsonStacError::Other("Failed to acquire read lock".to_string()))?;
            if data.is_some() {
                return Ok(());
            }
        }

        // Not loaded, acquire write lock and load
        let mut data = self
            .cached_header_data
            .write()
            .map_err(|_| CityJsonStacError::Other("Failed to acquire write lock".to_string()))?;

        // Double-check after acquiring write lock
        if data.is_none() {
            let file = File::open(&self.file_path)?;
            let reader = BufReader::new(file);

            let fcb_reader = FcbReader::open(reader)
                .map_err(|e| CityJsonStacError::Other(format!("Failed to open FCB file: {}", e)))?;

            let header = fcb_reader.header();

            // Extract geographical extent
            let geographical_extent = header.geographical_extent().map(|ge| {
                let min = ge.min();
                let max = ge.max();
                (min.x(), min.y(), min.z(), max.x(), max.y(), max.z())
            });

            // Extract reference system code
            let reference_system_code = header.reference_system().map(|rs| rs.code());

            // Extract transform
            let transform = header.transform().map(|t| {
                let scale = t.scale();
                let translate = t.translate();
                (
                    scale.x(),
                    scale.y(),
                    scale.z(),
                    translate.x(),
                    translate.y(),
                    translate.z(),
                )
            });

            // Extract columns for attribute definitions
            let columns: Vec<FcbColumn> = header
                .columns()
                .map(|cols| {
                    cols.iter()
                        .map(|col| FcbColumn {
                            name: col.name().to_string(),
                            column_type: col.type_().0,
                            nullable: col.nullable(),
                            description: col.description().map(|d| d.to_string()),
                        })
                        .collect()
                })
                .unwrap_or_default();

            // Extract metadata using the deserializer
            // Note: fcb_core::deserializer::to_cj_metadata might not be public,
            // so we construct metadata from available header fields
            let metadata_json = Self::extract_metadata_from_header(&header);

            // Get version
            let version = header.version().to_string();

            // Extract extensions
            let mut extensions = Vec::new();
            if let Some(extensions_vec) = header.extensions() {
                for extension in extensions_vec.iter() {
                    if let Some(url) = extension.url() {
                        extensions.push(url.to_string());
                    }
                }
            }
            extensions.sort();

            *data = Some(CachedHeaderData {
                version,
                geographical_extent,
                reference_system_code,
                transform,
                columns,
                metadata_json,
                extensions,
            });
        }
        Ok(())
    }

    /// Extract metadata as JSON from the header
    fn extract_metadata_from_header(header: &fcb_core::fb::Header) -> Option<serde_json::Value> {
        let mut metadata = serde_json::Map::new();

        // Add identifier if present
        if let Some(identifier) = header.identifier() {
            metadata.insert(
                "identifier".to_string(),
                serde_json::Value::String(identifier.to_string()),
            );
        }

        // Add title if present
        if let Some(title) = header.title() {
            metadata.insert(
                "title".to_string(),
                serde_json::Value::String(title.to_string()),
            );
        }

        // Add reference date if present
        if let Some(ref_date) = header.reference_date() {
            metadata.insert(
                "referenceDate".to_string(),
                serde_json::Value::String(ref_date.to_string()),
            );
        }

        // Add point of contact if present
        if let Some(poc_name) = header.poc_contact_name() {
            let mut poc = serde_json::Map::new();
            poc.insert(
                "contactName".to_string(),
                serde_json::Value::String(poc_name.to_string()),
            );

            if let Some(contact_type) = header.poc_contact_type() {
                poc.insert(
                    "contactType".to_string(),
                    serde_json::Value::String(contact_type.to_string()),
                );
            }
            if let Some(role) = header.poc_role() {
                poc.insert(
                    "role".to_string(),
                    serde_json::Value::String(role.to_string()),
                );
            }
            if let Some(email) = header.poc_email() {
                poc.insert(
                    "email".to_string(),
                    serde_json::Value::String(email.to_string()),
                );
            }
            if let Some(phone) = header.poc_phone() {
                poc.insert(
                    "phone".to_string(),
                    serde_json::Value::String(phone.to_string()),
                );
            }
            if let Some(website) = header.poc_website() {
                poc.insert(
                    "website".to_string(),
                    serde_json::Value::String(website.to_string()),
                );
            }

            metadata.insert("pointOfContact".to_string(), serde_json::Value::Object(poc));
        }

        // Add geographical extent
        if let Some(ge) = header.geographical_extent() {
            let min = ge.min();
            let max = ge.max();
            metadata.insert(
                "geographicalExtent".to_string(),
                serde_json::json!([min.x(), min.y(), min.z(), max.x(), max.y(), max.z()]),
            );
        }

        // Add reference system
        if let Some(rs) = header.reference_system() {
            let epsg_code = rs.code();
            // Construct the CityJSON-style reference system URL
            let ref_system_url = format!("https://www.opengis.net/def/crs/EPSG/0/{}", epsg_code);
            metadata.insert(
                "referenceSystem".to_string(),
                serde_json::Value::String(ref_system_url),
            );
        }

        if metadata.is_empty() {
            None
        } else {
            Some(serde_json::Value::Object(metadata))
        }
    }

    /// Execute a closure with access to the loaded header data
    fn with_header_data<T, F>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&CachedHeaderData) -> Result<T>,
    {
        self.ensure_loaded()?;
        let data = self
            .cached_header_data
            .read()
            .map_err(|_| CityJsonStacError::Other("Failed to acquire read lock".to_string()))?;
        let header_data = data
            .as_ref()
            .expect("data should be loaded after ensure_loaded");
        f(header_data)
    }

    /// Ensure streamed data is loaded and cached
    fn ensure_streamed_data_loaded(&self) -> Result<()> {
        // First check if already loaded with a read lock (cheaper)
        {
            let data = self
                .cached_streamed_data
                .read()
                .map_err(|_| CityJsonStacError::Other("Failed to acquire read lock".to_string()))?;
            if data.is_some() {
                return Ok(());
            }
        }

        // Not loaded, acquire write lock and load
        let mut data = self
            .cached_streamed_data
            .write()
            .map_err(|_| CityJsonStacError::Other("Failed to acquire write lock".to_string()))?;

        // Double-check after acquiring write lock
        if data.is_none() {
            let (lods, city_object_types, city_object_count) =
                self.stream_extract_lods_and_types_inner()?;
            *data = Some(CachedStreamedData {
                lods,
                city_object_types,
                city_object_count,
            });
        }

        Ok(())
    }

    /// Stream through features to extract LODs, city object types, and count
    ///
    /// This method iterates through all features in the FCB file in a streaming
    /// fashion, extracting unique LODs, city object types, and counting all
    /// CityObjects without loading all features into memory at once.
    fn stream_extract_lods_and_types_inner(&self) -> Result<(Vec<String>, Vec<String>, usize)> {
        let file = File::open(&self.file_path)?;
        let reader = BufReader::new(file);

        let fcb_reader = FcbReader::open(reader)
            .map_err(|e| CityJsonStacError::Other(format!("Failed to open FCB file: {}", e)))?;

        // Use BTreeSet for automatic deduplication and sorting
        let mut lods: BTreeSet<String> = BTreeSet::new();
        let mut types: BTreeSet<String> = BTreeSet::new();
        // Count all CityObjects (note: a single feature can have multiple objects)
        let mut city_object_count: usize = 0;

        // Use the sequential (streaming) iterator
        let mut feature_iter = fcb_reader
            .select_all_seq()
            .map_err(|e| CityJsonStacError::Other(format!("Failed to select features: {}", e)))?;

        // Stream through features one at a time
        while let Some(iter) = feature_iter
            .next()
            .map_err(|e| CityJsonStacError::Other(format!("Failed to read feature: {}", e)))?
        {
            let feature = iter.cur_feature();

            // Extract from city objects within this feature
            if let Some(objects) = feature.objects() {
                for obj in objects.iter() {
                    // Count this CityObject
                    city_object_count += 1;

                    // Extract city object type
                    let obj_type = obj.type_();
                    // Handle extension types specially
                    if obj_type == CityObjectType::ExtensionObject {
                        if let Some(ext_type) = obj.extension_type() {
                            types.insert(ext_type.to_string());
                        } else {
                            types.insert("ExtensionObject".to_string());
                        }
                    } else if let Some(name) = obj_type.variant_name() {
                        types.insert(name.to_string());
                    }

                    // Extract LODs from geometry
                    if let Some(geometries) = obj.geometry() {
                        for geom in geometries.iter() {
                            if let Some(lod) = geom.lod() {
                                lods.insert(lod.to_string());
                            }
                        }
                    }
                }
            }
        }

        // Convert to Vec (BTreeSet is already sorted)
        Ok((
            lods.into_iter().collect(),
            types.into_iter().collect(),
            city_object_count,
        ))
    }

    /// Execute a closure with access to the loaded streamed data
    fn with_streamed_data<T, F>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&CachedStreamedData) -> Result<T>,
    {
        self.ensure_streamed_data_loaded()?;
        let data = self
            .cached_streamed_data
            .read()
            .map_err(|_| CityJsonStacError::Other("Failed to acquire read lock".to_string()))?;
        let streamed_data = data
            .as_ref()
            .expect("data should be loaded after ensure_streamed_data_loaded");
        f(streamed_data)
    }
}

/// Map FCB ColumnType to AttributeType
fn map_column_type(column_type: u8) -> AttributeType {
    match column_type {
        t if t == ColumnType::Byte.0 => AttributeType::Number,
        t if t == ColumnType::UByte.0 => AttributeType::Number,
        t if t == ColumnType::Bool.0 => AttributeType::Boolean,
        t if t == ColumnType::Short.0 => AttributeType::Number,
        t if t == ColumnType::UShort.0 => AttributeType::Number,
        t if t == ColumnType::Int.0 => AttributeType::Number,
        t if t == ColumnType::UInt.0 => AttributeType::Number,
        t if t == ColumnType::Long.0 => AttributeType::Number,
        t if t == ColumnType::ULong.0 => AttributeType::Number,
        t if t == ColumnType::Float.0 => AttributeType::Number,
        t if t == ColumnType::Double.0 => AttributeType::Number,
        t if t == ColumnType::String.0 => AttributeType::String,
        t if t == ColumnType::Json.0 => AttributeType::Object,
        t if t == ColumnType::DateTime.0 => AttributeType::Date,
        _ => AttributeType::String, // Default fallback
    }
}

impl CityModelMetadataReader for FlatCityBufReader {
    fn bbox(&self) -> Result<BBox3D> {
        self.with_header_data(|data| {
            data.geographical_extent
                .map(|(xmin, ymin, zmin, xmax, ymax, zmax)| {
                    BBox3D::new(xmin, ymin, zmin, xmax, ymax, zmax)
                })
                .ok_or_else(|| {
                    CityJsonStacError::MetadataError(
                        "No geographical extent found in FCB header".to_string(),
                    )
                })
        })
    }

    fn crs(&self) -> Result<CRS> {
        self.with_header_data(|data| {
            if let Some(epsg_code) = data.reference_system_code {
                if epsg_code > 0 {
                    return Ok(CRS::from_epsg(epsg_code as u32));
                }
            }
            // Default to WGS84 if no CRS found
            Ok(CRS::default())
        })
    }

    fn lods(&self) -> Result<Vec<String>> {
        // Use cached streamed data
        self.with_streamed_data(|data| Ok(data.lods.clone()))
    }

    fn city_object_types(&self) -> Result<Vec<String>> {
        // Use cached streamed data
        self.with_streamed_data(|data| Ok(data.city_object_types.clone()))
    }

    fn city_object_count(&self) -> Result<usize> {
        // Use cached streamed data to get the actual number of CityObjects,
        // which may be higher than the number of FCB features (rows).
        // A single FCB feature can contain multiple CityObjects.
        self.with_streamed_data(|data| Ok(data.city_object_count))
    }

    fn attributes(&self) -> Result<Vec<AttributeDefinition>> {
        self.with_header_data(|data| {
            let mut attributes: Vec<AttributeDefinition> = data
                .columns
                .iter()
                .map(|col| {
                    let attr_type = map_column_type(col.column_type);
                    let mut attr_def = AttributeDefinition::new(&col.name, attr_type);

                    // nullable = true means required = false
                    attr_def = attr_def.with_required(!col.nullable);

                    if let Some(ref desc) = col.description {
                        attr_def = attr_def.with_description(desc);
                    }

                    attr_def
                })
                .collect();

            attributes.sort_by(|a, b| a.name.cmp(&b.name));
            Ok(attributes)
        })
    }

    fn encoding(&self) -> &'static str {
        "FlatCityBuf"
    }

    fn version(&self) -> Result<String> {
        self.with_header_data(|data| Ok(data.version.clone()))
    }

    fn file_path(&self) -> &Path {
        &self.file_path
    }

    fn transform(&self) -> Result<Option<Transform>> {
        self.with_header_data(|data| {
            Ok(data.transform.map(
                |(scale_x, scale_y, scale_z, translate_x, translate_y, translate_z)| {
                    Transform::new(
                        [scale_x, scale_y, scale_z],
                        [translate_x, translate_y, translate_z],
                    )
                },
            ))
        })
    }

    fn metadata(&self) -> Result<Option<serde_json::Value>> {
        self.with_header_data(|data| Ok(data.metadata_json.clone()))
    }

    fn extensions(&self) -> Result<Vec<String>> {
        self.with_header_data(|data| Ok(data.extensions.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_column_type() {
        assert_eq!(map_column_type(ColumnType::String.0), AttributeType::String);
        assert_eq!(map_column_type(ColumnType::Int.0), AttributeType::Number);
        assert_eq!(map_column_type(ColumnType::Bool.0), AttributeType::Boolean);
        assert_eq!(map_column_type(ColumnType::Json.0), AttributeType::Object);
    }

    #[test]
    fn test_reader_file_not_found() {
        let result = FlatCityBufReader::new(Path::new("nonexistent.fcb"));
        assert!(result.is_err());
    }
}
