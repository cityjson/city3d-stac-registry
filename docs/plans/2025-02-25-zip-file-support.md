# ZIP File Support Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add support for ZIP archives containing CityJSON, CityJSONSeq, or CityGML files, generating a single STAC Item with aggregated metadata.

**Architecture:** Create a `ZipReader` that extracts ZIP files to a temporary directory, discovers all supported format files inside, creates appropriate inner readers, and aggregates their metadata. The STAC Item's asset points to the ZIP URL with `application/zip` media type.

**Tech Stack:** Rust, `zip` crate v2.2, `tempfile` crate, existing `CityModelMetadataReader` trait

---

## Task 1: Add zip dependency

**Files:**
- Modify: `Cargo.toml`

**Step 1: Add zip dependency to Cargo.toml**

Find the dependencies section and add the zip crate:

```toml
[dependencies]
# ... existing dependencies ...
zip = "2.2"
```

**Step 2: Verify the change**

```bash
cargo check
```

Expected: No errors, dependency resolves successfully.

**Step 3: Commit**

```bash
git add Cargo.toml
git commit -m "feat: add zip crate dependency"
```

---

## Task 2: Create ZipReader structure

**Files:**
- Create: `src/reader/zip.rs`

**Step 1: Write the failing test**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_zip_reader_not_streamable() {
        // Create a minimal valid ZIP file
        let mut temp_zip = NamedTempFile::new().unwrap();
        let mut zip = zip::ZipWriter::new(temp_zip.as_file());
        zip.finish().unwrap();

        let reader = ZipReader::new(temp_zip.path()).unwrap();
        assert!(!reader.streamable());
    }
}
```

**Step 2: Run test to verify it fails**

```bash
cargo test --lib zip_reader
```

Expected: FAIL with "cannot find `ZipReader` in this scope"

**Step 3: Write minimal ZipReader struct**

Create `src/reader/zip.rs`:

```rust
//! ZIP archive reader for CityJSON/CityGML files
//!
//! Extracts ZIP archives and aggregates metadata from all supported files inside.

use crate::error::{CityJsonStacError, Result};
use crate::metadata::AttributeDefinition;
use crate::reader::{get_reader, CityModelMetadataReader};
use crate::metadata::BBox3D;
use crate::metadata::CRS;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::RwLock;
use tempfile::TempDir;

/// Reader for ZIP archives containing CityJSON/CityGML files
pub struct ZipReader {
    file_path: PathBuf,
    temp_dir: TempDir,
    inner_readers: Vec<Box<dyn CityModelMetadataReader>>,
    metadata: RwLock<Option<ZipMetadata>>,
}

/// Aggregated metadata from all files in the ZIP
#[derive(Debug)]
struct ZipMetadata {
    bbox: Option<BBox3D>,
    city_object_count: usize,
    city_object_types: BTreeSet<String>,
    lods: BTreeSet<String>,
    attributes: Vec<AttributeDefinition>,
    primary_encoding: &'static str,
    version: String,
    crs: Option<CRS>,
    has_textures: bool,
    has_materials: bool,
    has_semantic_surfaces: bool,
}

impl ZipReader {
    /// Create a new ZIP reader
    pub fn new(file_path: &Path) -> Result<Self> {
        if !file_path.exists() {
            return Err(CityJsonStacError::IoError(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("File not found: {}", file_path.display()),
            )));
        }

        // Create temporary directory for extraction
        let temp_dir = TempDir::new()?;

        // Extract ZIP to temp directory
        Self::extract_zip(file_path, temp_dir.path())?;

        let mut reader = Self {
            file_path: file_path.to_path_buf(),
            temp_dir,
            inner_readers: Vec::new(),
            metadata: RwLock::new(None),
        };

        // Discover and create inner readers
        reader.inner_readers = reader.discover_inner_readers()?;

        if reader.inner_readers.is_empty() {
            return Err(CityJsonStacError::InvalidCityJson(
                "No CityJSON/CityGML files found in ZIP".to_string(),
            ));
        }

        Ok(reader)
    }

    /// Extract ZIP file to directory
    fn extract_zip(zip_path: &Path, dest_dir: &Path) -> Result<()> {
        let file = std::fs::File::open(zip_path)?;
        let mut archive = zip::ZipArchive::new(file)?;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let outpath = dest_dir.join(file.name());

            if file.name().ends_with('/') {
                std::fs::create_dir_all(&outpath)?;
            } else {
                if let Some(parent) = outpath.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                let mut outfile = std::fs::File::create(&outpath)?;
                std::io::copy(&mut file, &mut outfile)?;
            }
        }

        Ok(())
    }

    /// Discover all supported files in extracted directory
    fn discover_inner_readers(&self) -> Result<Vec<Box<dyn CityModelMetadataReader>>> {
        let mut readers = Vec::new();

        // Walk the extracted directory
        fn walk_dir(
            dir: &Path,
            readers: &mut Vec<Box<dyn CityModelMetadataReader>>,
        ) -> Result<()> {
            for entry in std::fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();

                if path.is_dir() {
                    walk_dir(&path, readers)?;
                } else {
                    // Try to create a reader for this file
                    if let Ok(reader) = get_reader(&path) {
                        log::debug!("Found supported file in ZIP: {:?}", path);
                        readers.push(reader);
                    }
                }
            }
            Ok(())
        }

        walk_dir(self.temp_dir.path(), &mut readers)?;
        Ok(readers)
    }

    /// Aggregate metadata from all inner readers
    fn aggregate_metadata(&self) -> Result<ZipMetadata> {
        let mut city_object_count = 0;
        let mut city_object_types = BTreeSet::new();
        let mut lods = BTreeSet::new();
        let mut attributes = BTreeSet::new();
        let mut has_textures = false;
        let mut has_materials = false;
        let mut has_semantic_surfaces = false;

        // For bbox, we need to merge extents
        let mut min_x = f64::MAX;
        let mut min_y = f64::MAX;
        let mut min_z = f64::MAX;
        let mut max_x = f64::MIN;
        let mut max_y = f64::MIN;
        let mut max_z = f64::MIN;
        let mut has_bbox = false;

        let primary_encoding = self.inner_readers.first()
            .map(|r| r.encoding())
            .unwrap_or("CityJSON");

        let mut version = String::new();
        let mut crs = None;

        for reader in &self.inner_readers {
            // Count city objects
            if let Ok(count) = reader.city_object_count() {
                city_object_count += count;
            }

            // Collect city object types
            if let Ok(types) = reader.city_object_types() {
                city_object_types.extend(types);
            }

            // Collect LODs
            if let Ok(reader_lods) = reader.lods() {
                lods.extend(reader_lods);
            }

            // Collect attributes
            if let Ok(reader_attrs) = reader.attributes() {
                for attr in reader_attrs {
                    attributes.insert(attr);
                }
            }

            // Check for textures/materials/semantic surfaces
            if let Ok(true) = reader.textures() {
                has_textures = true;
            }
            if let Ok(true) = reader.materials() {
                has_materials = true;
            }
            if let Ok(true) = reader.semantic_surfaces() {
                has_semantic_surfaces = true;
            }

            // Merge bbox
            if let Ok(bbox) = reader.bbox() {
                has_bbox = true;
                min_x = min_x.min(bbox.min_x);
                min_y = min_y.min(bbox.min_y);
                min_z = min_z.min(bbox.min_z);
                max_x = max_x.max(bbox.max_x);
                max_y = max_y.max(bbox.max_y);
                max_z = max_z.max(bbox.max_z);
            }

            // Get version and CRS from first reader
            if version.is_empty() {
                if let Ok(v) = reader.version() {
                    version = v;
                }
            }
            if crs.is_none() {
                if let Ok(c) = reader.crs() {
                    crs = Some(c);
                }
            }
        }

        let bbox = if has_bbox {
            Some(BBox3D::new(min_x, min_y, min_z, max_x, max_y, max_z))
        } else {
            None
        };

        let attributes: Vec<_> = attributes.into_iter().collect();

        Ok(ZipMetadata {
            bbox,
            city_object_count,
            city_object_types,
            lods,
            attributes,
            primary_encoding,
            version,
            crs: crs.unwrap_or_default(),
            has_textures,
            has_materials,
            has_semantic_surfaces,
        })
    }

    /// Lazy load metadata
    fn ensure_loaded(&self) -> Result<()> {
        {
            let metadata = self.metadata.read()
                .map_err(|_| CityJsonStacError::Other("Failed to acquire read lock".to_string()))?;
            if metadata.is_some() {
                return Ok(());
            }
        }

        let mut metadata = self.metadata.write()
            .map_err(|_| CityJsonStacError::Other("Failed to acquire write lock".to_string()))?;

        if metadata.is_none() {
            *metadata = Some(self.aggregate_metadata()?);
        }

        Ok(())
    }
}

impl CityModelMetadataReader for ZipReader {
    fn bbox(&self) -> Result<BBox3D> {
        self.ensure_loaded()?;
        let metadata = self.metadata.read()
            .map_err(|_| CityJsonStacError::Other("Failed to acquire read lock".to_string()))?;
        metadata.as_ref()
            .and_then(|m| m.bbox.clone())
            .ok_or_else(|| CityJsonStacError::MetadataError("BBox not found".to_string()))
    }

    fn crs(&self) -> Result<CRS> {
        self.ensure_loaded()?;
        let metadata = self.metadata.read()
            .map_err(|_| CityJsonStacError::Other("Failed to acquire read lock".to_string()))?;
        Ok(metadata.as_ref().unwrap().crs.clone())
    }

    fn lods(&self) -> Result<Vec<String>> {
        self.ensure_loaded()?;
        let metadata = self.metadata.read()
            .map_err(|_| CityJsonStacError::Other("Failed to acquire read lock".to_string()))?;
        Ok(metadata.as_ref().unwrap().lods.iter().cloned().collect())
    }

    fn city_object_types(&self) -> Result<Vec<String>> {
        self.ensure_loaded()?;
        let metadata = self.metadata.read()
            .map_err(|_| CityJsonStacError::Other("Failed to acquire read lock".to_string()))?;
        Ok(metadata.as_ref().unwrap().city_object_types.iter().cloned().collect())
    }

    fn city_object_count(&self) -> Result<usize> {
        self.ensure_loaded()?;
        let metadata = self.metadata.read()
            .map_err(|_| CityJsonStacError::Other("Failed to acquire read lock".to_string()))?;
        Ok(metadata.as_ref().unwrap().city_object_count)
    }

    fn attributes(&self) -> Result<Vec<AttributeDefinition>> {
        self.ensure_loaded()?;
        let metadata = self.metadata.read()
            .map_err(|_| CityJsonStacError::Other("Failed to acquire read lock".to_string()))?;
        Ok(metadata.as_ref().unwrap().attributes.clone())
    }

    fn encoding(&self) -> &'static str {
        // Return the internal format (from first file found)
        // Priority will be determined by the order files are discovered
        "CityJSON" // Default, will be overridden by primary_encoding in actual impl
    }

    fn version(&self) -> Result<String> {
        self.ensure_loaded()?;
        let metadata = self.metadata.read()
            .map_err(|_| CityJsonStacError::Other("Failed to acquire read lock".to_string()))?;
        Ok(metadata.as_ref().unwrap().version.clone())
    }

    fn file_path(&self) -> &Path {
        &self.file_path
    }

    fn transform(&self) -> Result<Option<crate::metadata::Transform>> {
        Ok(None) // ZIP wrapper doesn't use vertex compression
    }

    fn metadata(&self) -> Result<Option<serde_json::Value>> {
        Ok(None)
    }

    fn extensions(&self) -> Result<Vec<String>> {
        Ok(Vec::new())
    }

    fn semantic_surfaces(&self) -> Result<bool> {
        self.ensure_loaded()?;
        let metadata = self.metadata.read()
            .map_err(|_| CityJsonStacError::Other("Failed to acquire read lock".to_string()))?;
        Ok(metadata.as_ref().unwrap().has_semantic_surfaces)
    }

    fn textures(&self) -> Result<bool> {
        self.ensure_loaded()?;
        let metadata = self.metadata.read()
            .map_err(|_| CityJsonStacError::Other("Failed to acquire read lock".to_string()))?;
        Ok(metadata.as_ref().unwrap().has_textures)
    }

    fn materials(&self) -> Result<bool> {
        self.ensure_loaded()?;
        let metadata = self.metadata.read()
            .map_err(|_| CityJsonStacError::Other("Failed to acquire read lock".to_string()))?;
        Ok(metadata.as_ref().unwrap().has_materials)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_zip_reader_not_streamable() {
        // Create a minimal valid ZIP file
        let mut temp_zip = NamedTempFile::new().unwrap();
        let mut zip = zip::ZipWriter::new(temp_zip.as_file());
        zip.finish().unwrap();

        let reader = ZipReader::new(temp_zip.path()).unwrap();
        assert!(!reader.streamable());
    }
}
```

**Step 4: Run test to verify it passes**

```bash
cargo test --lib zip_reader
```

Expected: PASS

**Step 5: Commit**

```bash
git add src/reader/zip.rs
git commit -m "feat: add ZipReader struct and basic implementation"
```

---

## Task 3: Fix encoding() method to use primary_encoding

**Files:**
- Modify: `src/reader/zip.rs`

**Step 1: Write test for encoding method**

Add to `src/reader/zip.rs` tests:

```rust
#[test]
fn test_zip_reader_encoding() {
    // This test will be implemented once we have a way to create test ZIPs with content
    // For now, we'll verify the method exists
    let mut temp_zip = NamedTempFile::new().unwrap();
    let mut zip = zip::ZipWriter::new(temp_zip.as_file());

    // Add an empty directory to make it a valid ZIP
    zip.add_directory("test/", zip::write::FileOptions::default()).unwrap();
    zip.finish().unwrap();

    // Note: This will fail with "No CityJSON/CityGML files found" until we add test data
    // The test is mainly to verify the method signature
}
```

**Step 2: Run test to see it compile**

```bash
cargo check
```

**Step 3: Fix the encoding() method**

Replace the encoding method in `ZipReader` implementation:

```rust
fn encoding(&self) -> &'static str {
    // Return the internal format from first file found
    // Priority is determined by discovery order (get_reader priority)
    if let Ok(metadata) = self.metadata.read() {
        if let Some(ref m) = *metadata {
            return m.primary_encoding;
        }
    }
    "CityJSON" // Fallback
}
```

**Step 4: Verify it compiles**

```bash
cargo check
```

Expected: No errors

**Step 5: Commit**

```bash
git add src/reader/zip.rs
git commit -m "fix: use primary_encoding for ZipReader::encoding()"
```

---

## Task 4: Register ZipReader in reader module

**Files:**
- Modify: `src/reader/mod.rs`

**Step 1: Write test for ZIP file detection**

Add to `src/reader/mod.rs` tests:

```rust
#[test]
fn test_get_reader_zip_file() {
    use std::io::Write;
    use zip::write::FileOptions;

    let mut temp_zip = tempfile::NamedTempFile::new().unwrap();
    let mut zip = zip::ZipWriter::new(temp_zip.as_file());

    // Add a minimal CityJSON file to the ZIP
    let cityjson = r#"{
        "type": "CityJSON",
        "version": "1.1",
        "CityObjects": {},
        "vertices": []
    }"#;

    let options = FileOptions::default()
        .compression_method(zip::CompressionMethod::Stored);
    zip.start_file("test.json", options).unwrap();
    zip.write_all(cityjson.as_bytes()).unwrap();
    zip.finish().unwrap();

    let reader = get_reader(temp_zip.path());
    assert!(reader.is_ok());
    assert_eq!(reader.unwrap().encoding(), "CityJSON");
}
```

**Step 2: Run test to verify it fails**

```bash
cargo test test_get_reader_zip_file
```

Expected: FAIL with "Unsupported file extension: zip"

**Step 3: Add ZipReader to mod.rs**

At the top of `src/reader/mod.rs`, add:

```rust
pub mod zip;
pub use zip::ZipReader;
```

**Step 4: Add .zip case to get_reader() function**

Update the `get_reader()` function:

```rust
pub fn get_reader(path: &Path) -> Result<Box<dyn CityModelMetadataReader>> {
    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .ok_or_else(|| CityJsonStacError::InvalidCityJson("No file extension".to_string()))?;

    match extension {
        "zip" => Ok(Box::new(ZipReader::new(path)?)),
        "json" => Ok(Box::new(CityJSONReader::new(path)?)),
        "jsonl" => Ok(Box::new(CityJSONSeqReader::new(path)?)),
        "fcb" => Ok(Box::new(FlatCityBufReader::new(path)?)),
        "gml" | "xml" => {
            if is_citygml(path)? {
                Ok(Box::new(CityGMLReader::new(path)?))
            } else {
                Err(CityJsonStacError::UnsupportedFormat(format!(
                    "File is not a valid CityGML file: {extension}"
                )))
            }
        }
        _ => Err(CityJsonStacError::InvalidCityJson(format!(
            "Unsupported file extension: {extension}",
        ))),
    }
}
```

**Step 5: Run test to verify it passes**

```bash
cargo test test_get_reader_zip_file
```

Expected: PASS

**Step 6: Commit**

```bash
git add src/reader/mod.rs
git commit -m "feat: add ZIP file support to get_reader()"
```

---

## Task 5: Add remote ZIP file support

**Files:**
- Modify: `src/reader/mod.rs`

**Step 1: Write test for remote ZIP**

Add to `src/reader/mod.rs` tests:

```rust
#[tokio::test]
async fn test_get_reader_from_source_remote_zip() {
    // This test would require a mock HTTP server
    // For now, we'll verify the extension is accepted
    let source = InputSource::Remote("https://example.com/data.zip".to_string());
    let result = get_reader_from_source(&source).await;

    // Should not fail with "Unsupported remote file extension"
    // (will fail with actual download error, which is expected)
    match result {
        Err(CityJsonStacError::InvalidCityJson(msg)) if msg.contains("Unsupported remote file extension") => {
            panic!("ZIP extension should be supported");
        }
        _ => {} // Expected: download error or other
    }
}
```

**Step 2: Run test to verify it fails**

```bash
cargo test test_get_reader_from_source_remote_zip
```

Expected: FAIL with "Unsupported remote file extension: zip"

**Step 3: Add .zip case to get_reader_from_source()**

Update the `get_reader_from_source()` function in `src/reader/mod.rs`:

```rust
pub async fn get_reader_from_source(
    source: &InputSource,
) -> Result<Box<dyn CityModelMetadataReader>> {
    match source {
        InputSource::Local(path) => get_reader(path),
        InputSource::Remote(url) => {
            // Validate extension before downloading
            let extension = extract_extension_from_url(url)?;
            match extension.as_str() {
                "json" | "jsonl" | "cjseq" | "gml" | "xml" | "zip" => {}
                _ => {
                    return Err(CityJsonStacError::InvalidCityJson(format!(
                        "Unsupported remote file extension: {extension}. Supported: .json, .jsonl, .cjseq, .gml, .xml, .zip",
                    )));
                }
            }

            let filename = url_filename(url);
            let virtual_path = PathBuf::from(&filename);

            match extension.as_str() {
                "json" => {
                    log::info!("Downloading remote CityJSON file: {}", url);
                    let bytes = download_from_url(url).await?;
                    let content = String::from_utf8(bytes.to_vec()).map_err(|e| {
                        CityJsonStacError::Other(format!("Remote file is not valid UTF-8: {e}"))
                    })?;
                    log::debug!("Downloaded {} bytes for {}", content.len(), filename);
                    Ok(Box::new(CityJSONReader::from_content(
                        &content,
                        virtual_path,
                    )?))
                }
                "jsonl" | "cjseq" => {
                    log::info!("Streaming remote CityJSONSeq file: {}", url);
                    Ok(Box::new(
                        CityJSONSeqReader::from_url_stream(url, virtual_path).await?,
                    ))
                }
                "gml" | "xml" => {
                    log::info!("Downloading remote CityGML file: {}", url);
                    let bytes = download_from_url(url).await?;
                    let mut temp_file = tempfile::Builder::new()
                        .suffix(&format!(".{}", extension))
                        .tempfile()?;
                    use std::io::Write;
                    temp_file.write_all(&bytes)?;
                    let path = temp_file.path().to_path_buf();
                    let reader =
                        CityGMLReader::new(&path)?.with_temp_path(temp_file.into_temp_path());
                    Ok(Box::new(reader))
                }
                "zip" => {
                    log::info!("Downloading remote ZIP file: {}", url);
                    let bytes = download_from_url(url).await?;
                    let mut temp_file = tempfile::Builder::new()
                        .suffix(".zip")
                        .tempfile()?;
                    use std::io::Write;
                    temp_file.write_all(&bytes)?;
                    let path = temp_file.path().to_path_buf();
                    let reader = ZipReader::new(&path)?;
                    // Keep temp file alive for ZipReader's lifetime
                    std::mem::forget(temp_file);
                    Ok(Box::new(reader))
                }
                _ => unreachable!("extension already validated above"),
            }
        }
    }
}
```

**Step 4: Run test to verify it passes**

```bash
cargo test test_get_reader_from_source_remote_zip
```

Expected: PASS

**Step 5: Commit**

```bash
git add src/reader/mod.rs
git commit -m "feat: add remote ZIP file support"
```

---

## Task 6: Set application/zip media type for ZIP sources

**Files:**
- Modify: `src/stac/item.rs`

**Step 1: Update data_asset to use zip media type**

Modify the `from_file` and `from_file_with_format_suffix` methods in `StacItemBuilder` to detect ZIP files:

```rust
/// Helper to create item from file path
pub fn from_file(
    file_path: &Path,
    reader: &dyn CityModelMetadataReader,
    base_url: Option<&str>,
) -> Result<Self> {
    let id = file_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();

    let mut builder = Self::new(id);

    // Set bbox (transformed to WGS84 for STAC compliance)
    if let Ok(bbox) = reader.bbox() {
        let crs = reader.crs().unwrap_or_default();
        let wgs84_bbox = bbox.to_wgs84(&crs)?;
        builder = builder.bbox(wgs84_bbox).geometry_from_bbox();
    }

    // Add CityJSON metadata
    builder = builder.cityjson_metadata(reader)?;

    // Add file size (File Extension)
    if let Ok(metadata) = std::fs::metadata(file_path) {
        builder = builder.file_size(metadata.len());
    }

    // Add data asset - detect ZIP files
    let is_zip = file_path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e == "zip")
        .unwrap_or(false);

    let media_type = if is_zip {
        "application/zip"
    } else {
        match reader.encoding() {
            "CityJSON" => "application/json",
            "CityJSONSeq" => "application/json-seq",
            "CityGML" => "application/gml+xml",
            "FlatCityBuf" => "application/octet-stream",
            _ => "application/octet-stream",
        }
    };

    // Generate asset href based on base_url
    let file_name = file_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("data");

    let href = match base_url {
        Some(base) => {
            let normalized_base = if base.ends_with('/') {
                base.to_string()
            } else {
                format!("{base}/")
            };
            format!("{normalized_base}{file_name}")
        }
        None => file_name.to_string(),
    };

    builder = builder.data_asset(href, media_type);

    Ok(builder)
}
```

**Step 2: Update from_file_with_format_suffix similarly**

Apply the same ZIP detection to `from_file_with_format_suffix`:

```rust
pub fn from_file_with_format_suffix(
    file_path: &Path,
    reader: &dyn CityModelMetadataReader,
    base_url: Option<&str>,
) -> Result<Self> {
    let stem = file_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown");

    // Don't add format suffix for ZIP files (use stem directly)
    let is_zip = file_path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e == "zip")
        .unwrap_or(false);

    let id = if is_zip {
        stem.to_string()
    } else {
        let suffix = match reader.encoding() {
            "CityJSON" => "_cj",
            "CityJSONSeq" => "_cjseq",
            "FlatCityBuf" => "_fcb",
            _ => "",
        };
        format!("{stem}{suffix}")
    };

    let mut builder = Self::new(id);

    // Set bbox (transformed to WGS84 for STAC compliance)
    if let Ok(bbox) = reader.bbox() {
        let crs = reader.crs().unwrap_or_default();
        let wgs84_bbox = bbox.to_wgs84(&crs)?;
        builder = builder.bbox(wgs84_bbox).geometry_from_bbox();
    }

    // Add CityJSON metadata
    builder = builder.cityjson_metadata(reader)?;

    // Add file size (File Extension)
    if let Ok(metadata) = std::fs::metadata(file_path) {
        builder = builder.file_size(metadata.len());
    }

    // Add data asset - detect ZIP files
    let media_type = if is_zip {
        "application/zip"
    } else {
        match reader.encoding() {
            "CityJSON" => "application/json",
            "CityJSONSeq" => "application/json-seq",
            "CityGML" => "application/gml+xml",
            "FlatCityBuf" => "application/octet-stream",
            _ => "application/octet-stream",
        }
    };

    // Generate asset href based on base_url
    let file_name = file_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("data");

    let href = match base_url {
        Some(base) => {
            let normalized_base = if base.ends_with('/') {
                base.to_string()
            } else {
                format!("{base}/")
            };
            format!("{normalized_base}{file_name}")
        }
        None => file_name.to_string(),
    };

    builder = builder.data_asset(href, media_type);

    Ok(builder)
}
```

**Step 3: Verify compilation**

```bash
cargo check
```

Expected: No errors

**Step 4: Commit**

```bash
git add src/stac/item.rs
git commit -m "feat: use application/zip media type for ZIP sources"
```

---

## Task 7: Fix ZipReader temp file cleanup issue

**Files:**
- Modify: `src/reader/mod.rs`, `src/reader/zip.rs`

**Step 1: Update ZipReader to own its temp file**

Modify `src/reader/zip.rs`:

```rust
pub struct ZipReader {
    file_path: PathBuf,
    temp_dir: TempDir,
    _temp_file: Option<tempfile::TempPath>,  // For remote ZIPs
    inner_readers: Vec<Box<dyn CityModelMetadataReader>>,
    metadata: RwLock<Option<ZipMetadata>>,
}

impl ZipReader {
    pub fn new(file_path: &Path) -> Result<Self> {
        if !file_path.exists() {
            return Err(CityJsonStacError::IoError(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("File not found: {}", file_path.display()),
            )));
        }

        let temp_dir = TempDir::new()?;
        Self::extract_zip(file_path, temp_dir.path())?;

        Ok(Self {
            file_path: file_path.to_path_buf(),
            temp_dir,
            _temp_file: None,
            inner_readers: Vec::new(),
            metadata: RwLock::new(None),
        })
    }

    /// Create from a temporary file that will be cleaned up on drop
    pub fn from_temp_file(file_path: PathBuf, temp_path: tempfile::TempPath) -> Result<Self> {
        let temp_dir = TempDir::new()?;
        Self::extract_zip(&file_path, temp_dir.path())?;

        Ok(Self {
            file_path,
            temp_dir,
            _temp_file: Some(temp_path),
            inner_readers: Vec::new(),
            metadata: RwLock::new(None),
        })
    }

    // ... rest of methods unchanged
}
```

**Step 2: Update get_reader_from_source to use from_temp_file**

Modify `src/reader/mod.rs`:

```rust
"zip" => {
    log::info!("Downloading remote ZIP file: {}", url);
    let bytes = download_from_url(url).await?;
    let mut temp_file = tempfile::Builder::new()
        .suffix(".zip")
        .tempfile()?;
    use std::io::Write;
    temp_file.write_all(&bytes)?;
    let path = temp_file.path().to_path_buf();
    let reader = ZipReader::from_temp_file(path, temp_file.into_temp_path())?;
    Ok(Box::new(reader))
}
```

**Step 3: Verify compilation**

```bash
cargo check
```

Expected: No errors

**Step 4: Commit**

```bash
git add src/reader/zip.rs src/reader/mod.rs
git commit -m "fix: properly handle temp file cleanup for remote ZIPs"
```

---

## Task 8: Add comprehensive ZIP reader tests

**Files:**
- Modify: `src/reader/zip.rs`

**Step 1: Add helper to create test ZIP**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use zip::write::{FileOptions, SimpleFileOptions};

    fn create_test_zip_with_cityjson() -> NamedTempFile {
        let mut temp_zip = NamedTempFile::new().unwrap();
        let mut zip = zip::ZipWriter::new(temp_zip.as_file());

        let cityjson = r#"{
            "type": "CityJSON",
            "version": "1.1",
            "transform": {
                "scale": [0.01, 0.01, 0.01],
                "translate": [100000, 200000, 0]
            },
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
                        "boundaries": [[[[0,0,0]]]]
                    }]
                }
            },
            "vertices": [[0,0,0]]
        }"#;

        let options = SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);

        zip.start_file("data.city.json", options).unwrap();
        zip.write_all(cityjson.as_bytes()).unwrap();
        zip.finish().unwrap();

        temp_zip
    }

    // ... existing tests
}
```

**Step 2: Add test for metadata aggregation**

```rust
#[test]
fn test_zip_reader_aggregates_metadata() {
    let temp_zip = create_test_zip_with_cityjson();
    let reader = ZipReader::new(temp_zip.path()).unwrap();

    // Should have bbox from inner file
    let bbox = reader.bbox().unwrap();
    assert_eq!(bbox.min_x, 1.0);
    assert_eq!(bbox.max_x, 10.0);

    // Should have city object count
    let count = reader.city_object_count().unwrap();
    assert_eq!(count, 1);

    // Should have city object types
    let types = reader.city_object_types().unwrap();
    assert!(types.contains(&"Building".to_string()));

    // Should have LODs
    let lods = reader.lods().unwrap();
    assert!(lods.contains(&"2".to_string()));

    // Should have version
    let version = reader.version().unwrap();
    assert_eq!(version, "1.1");

    // Should have CRS
    let crs = reader.crs().unwrap();
    assert_eq!(crs.to_stac_epsg(), Some(7415));

    // Should have CityJSON encoding
    assert_eq!(reader.encoding(), "CityJSON");
}
```

**Step 3: Add test for empty ZIP**

```rust
#[test]
fn test_zip_reader_empty_zip() {
    let mut temp_zip = NamedTempFile::new().unwrap();
    let mut zip = zip::ZipWriter::new(temp_zip.as_file());
    zip.finish().unwrap();

    let result = ZipReader::new(temp_zip.path());
    assert!(result.is_err());
    match result.unwrap_err() {
        CityJsonStacError::InvalidCityJson(msg) => {
            assert!(msg.contains("No CityJSON/CityGML files found"));
        }
        _ => panic!("Expected InvalidCityJson error"),
    }
}
```

**Step 4: Run tests**

```bash
cargo test --lib zip_reader
```

Expected: All tests pass

**Step 5: Commit**

```bash
git add src/reader/zip.rs
git commit -m "test: add comprehensive ZipReader tests"
```

---

## Task 9: Update CLAUDE.md documentation

**Files:**
- Modify: `CLAUDE.md`

**Step 1: Document ZIP file support**

Add to the "Supported Input Formats" table:

```markdown
| Format                | Extension | Library                                  | Status |
| --------------------- | --------- | ---------------------------------------- | ------ |
| CityJSON              | `.json`   | `serde_json`                             | ✅     |
| CityJSONTextSequences | `.jsonl`  | `serde_json` (streaming)                 | ✅     |
| ZIP Archive           | `.zip`    | `zip` (with inner readers)               | ✅     |
| FlatCityBuf           | `.fcb`    | [flatcitybuf](https://github.com/cityjson/flatcitybuf) Rust crate | 🚧     |
```

**Step 2: Add ZIP section to documentation**

```markdown
## ZIP Archive Support

The CLI supports ZIP archives containing CityJSON, CityJSONSeq, or CityGML files.

### Behavior

- **Single Item**: A ZIP file generates one STAC Item (not a Collection)
- **Asset Href**: Points to the ZIP file URL
- **Asset Type**: `application/zip`
- **Metadata**: Aggregated from all supported files inside (bbox union, object count sum, LODs/types union)
- **city3d:encoding**: Reflects the internal format (CityJSON/CityGML/etc)

### Format Priority

When the ZIP contains mixed formats, the encoding is determined by priority:
1. CityJSON (.json)
2. CityJSONSeq (.jsonl)
3. CityGML (.gml, .xml)

### Example

```bash
# Local ZIP file
cityjson-stac item data.zip -o data_item.json

# Remote ZIP file
cityjson-stac item https://example.com/data.zip -o data_item.json

# With base URL
cityjson-stac item https://example.com/data.zip --base-url https://cdn.example.com -o data_item.json
```
```

**Step 3: Commit**

```bash
git add CLAUDE.md
git commit -m "docs: document ZIP file support"
```

---

## Summary

After completing all tasks, the cityjson-stac CLI will support ZIP archives containing CityJSON, CityJSONSeq, or CityGML files. The implementation follows the existing patterns and includes comprehensive tests.

**Key files created/modified:**
- `Cargo.toml` - Added `zip` dependency
- `src/reader/zip.rs` - New ZipReader implementation
- `src/reader/mod.rs` - Registered ZIP support
- `src/stac/item.rs` - Set `application/zip` media type
- `CLAUDE.md` - Documentation
