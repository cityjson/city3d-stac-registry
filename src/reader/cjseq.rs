//! CityJSON Text Sequences (CityJSONSeq) format reader
//!
//! Reads `.city.jsonl` or `.cjseq` files which contain JSON Text Sequences
//! as specified in the CityJSON 2.0 specification.
//!
//! The format consists of:
//! - First line: CityJSON header with metadata, transform, and empty CityObjects/vertices
//! - Subsequent lines: CityJSONFeature objects, each with their own vertices

use crate::error::{CityJsonStacError, Result};
use crate::metadata::{AttributeDefinition, BBox3D, Transform, CRS};
use crate::reader::CityModelMetadataReader;
use serde_json::Value;
use std::path::{Path, PathBuf};

/// Reader for CityJSON Text Sequences format files (.city.jsonl, .jsonl, .cjseq)
pub struct CityJSONSeqReader {
    file_path: PathBuf,

    data: AggregatedMetadata,
}

pub struct AggregatedMetadata {
    //TODO: to be implemented
}

impl Default for AggregatedMetadata {
    fn default() -> Self {
        Self::new()
    }
}

impl AggregatedMetadata {
    pub fn new() -> Self {
        Self {}
    }
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
            data: AggregatedMetadata::new(),
        })
    }
}

impl CityModelMetadataReader for CityJSONSeqReader {
    fn bbox(&self) -> Result<BBox3D> {
        todo!()
    }

    fn crs(&self) -> Result<CRS> {
        todo!()
    }

    fn lods(&self) -> Result<Vec<String>> {
        todo!()
    }

    fn city_object_types(&self) -> Result<Vec<String>> {
        todo!()
    }

    fn city_object_count(&self) -> Result<usize> {
        todo!()
    }

    fn attributes(&self) -> Result<Vec<AttributeDefinition>> {
        todo!()
    }

    fn encoding(&self) -> &'static str {
        todo!()
    }

    fn version(&self) -> Result<String> {
        todo!()
    }

    fn file_path(&self) -> &Path {
        todo!()
    }

    fn transform(&self) -> Result<Option<Transform>> {
        todo!()
    }

    fn metadata(&self) -> Result<Option<Value>> {
        todo!()
    }

    fn extensions(&self) -> Result<Vec<String>> {
        todo!()
    }

    fn semantic_surfaces(&self) -> Result<bool> {
        todo!()
    }

    fn textures(&self) -> Result<bool> {
        todo!()
    }

    fn materials(&self) -> Result<bool> {
        todo!()
    }
}

//Add comprehensive tests.
