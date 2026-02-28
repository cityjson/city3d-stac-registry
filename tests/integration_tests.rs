//! End-to-end integration tests

use city3d_stac::reader::{get_reader, CityJSONReader, CityModelMetadataReader};
use city3d_stac::stac::{StacCollectionBuilder, StacItemBuilder};
use std::path::Path;
use tempfile::tempdir;

/// Test data directory path
fn test_data_path(filename: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("data")
        .join(filename)
}

mod e2e_single_file_tests {
    use super::*;

    #[test]
    fn test_e2e_delft_cityjson_to_stac_item() {
        // Full workflow: read CityJSON -> build STAC item -> validate output
        let path = test_data_path("delft.city.json");

        // Step 1: Read the file
        let reader = get_reader(&path).expect("Failed to get reader");

        // Step 2: Build STAC item
        let item = StacItemBuilder::from_file(&path, reader.as_ref(), None)
            .expect("Failed to create item builder")
            .build()
            .expect("Failed to build item");

        // Step 3: Validate STAC item structure
        assert_eq!(item.stac_version, "1.1.0");
        assert_eq!(item.item_type, "Feature");
        assert!(!item.id.is_empty());

        // Validate bbox
        assert!(item.bbox.is_some());
        let bbox = item.bbox.unwrap();
        assert_eq!(bbox.len(), 6);

        // Validate geometry
        assert!(item.geometry.is_some());
        let geom = item.geometry.unwrap();
        assert_eq!(geom["type"], "Polygon");

        // Validate CityJSON extension properties
        // city3d:encoding is removed
        assert_eq!(item.properties["city3d:version"], "2.0");

        // Validate projection extension
        assert_eq!(item.properties["proj:epsg"], 7415);

        // Validate required STAC extensions
        assert!(item
            .stac_extensions
            .iter()
            .any(|e| e.contains("stac-city3d")));
        assert!(item
            .stac_extensions
            .iter()
            .any(|e| e.contains("projection")));

        // Validate assets
        assert!(item.assets.contains_key("data"));
    }

    #[test]
    fn test_e2e_railway_cityjson_to_stac_item() {
        let path = test_data_path("railway.city.json");

        let reader = get_reader(&path).expect("Failed to get reader");
        let item = StacItemBuilder::from_file(&path, reader.as_ref(), None)
            .expect("Failed to create builder")
            .build()
            .expect("Failed to build item");

        // Railway has city objects
        assert!(item.properties["city3d:city_objects"].as_u64().unwrap() > 0);

        // Railway has LODs
        assert!(item.properties.contains_key("city3d:lods"));

        // Railway has object types
        assert!(item.properties.contains_key("city3d:co_types"));
    }

    #[test]
    fn test_e2e_item_serialization() {
        let path = test_data_path("delft.city.json");
        let reader = get_reader(&path).expect("Failed to get reader");
        let item = StacItemBuilder::from_file(&path, reader.as_ref(), None)
            .expect("Failed to create builder")
            .build()
            .expect("Failed to build item");

        // Serialize to JSON
        let json = serde_json::to_string_pretty(&item).expect("Failed to serialize");

        // Deserialize back
        let deserialized: serde_json::Value =
            serde_json::from_str(&json).expect("Failed to deserialize");

        // Validate structure
        assert_eq!(deserialized["stac_version"], "1.1.0");
        assert_eq!(deserialized["type"], "Feature");
        // city3d:encoding removed
    }

    #[test]
    fn test_e2e_item_output_to_file() {
        let path = test_data_path("delft.city.json");
        let reader = get_reader(&path).expect("Failed to get reader");
        let item = StacItemBuilder::from_file(&path, reader.as_ref(), None)
            .expect("Failed to create builder")
            .build()
            .expect("Failed to build item");

        let temp = tempdir().expect("Failed to create temp dir");
        let output_path = temp.path().join("item.json");

        let json = serde_json::to_string_pretty(&item).expect("Failed to serialize");
        std::fs::write(&output_path, &json).expect("Failed to write file");

        // Verify file was created
        assert!(output_path.exists());

        // Verify content
        let content = std::fs::read_to_string(&output_path).expect("Failed to read file");
        let parsed: serde_json::Value = serde_json::from_str(&content).expect("Failed to parse");
        assert_eq!(parsed["stac_version"], "1.1.0");
    }
}

mod e2e_collection_tests {
    use super::*;

    #[test]
    fn test_e2e_build_collection_from_single_file() {
        let path = test_data_path("delft.city.json");
        let reader = CityJSONReader::new(&path).expect("Failed to create reader");

        let readers: Vec<Box<dyn CityModelMetadataReader>> = vec![Box::new(reader)];

        let collection = StacCollectionBuilder::new("test-collection")
            .title("Test Collection")
            .description("A test STAC collection")
            .aggregate_cityjson_metadata(&readers)
            .expect("Failed to aggregate metadata")
            .build()
            .expect("Failed to build collection");

        assert_eq!(collection.id, "test-collection");
        assert_eq!(collection.title, Some("Test Collection".to_string()));
        assert_eq!(collection.stac_version, "1.1.0");
        assert_eq!(collection.collection_type, "Collection");

        // Validate extent
        assert!(!collection.extent.spatial.bbox.is_empty());

        // Validate summaries
        assert!(collection.summaries.is_some());
    }

    #[test]
    fn test_e2e_build_collection_from_multiple_files() {
        let path1 = test_data_path("delft.city.json");
        let path2 = test_data_path("railway.city.json");

        let reader1 = CityJSONReader::new(&path1).expect("Failed to create reader 1");
        let reader2 = CityJSONReader::new(&path2).expect("Failed to create reader 2");

        let readers: Vec<Box<dyn CityModelMetadataReader>> =
            vec![Box::new(reader1), Box::new(reader2)];

        let collection = StacCollectionBuilder::new("multi-file-collection")
            .aggregate_cityjson_metadata(&readers)
            .expect("Failed to aggregate metadata")
            .build()
            .expect("Failed to build collection");

        // Extent should contain merged bbox
        let bbox = &collection.extent.spatial.bbox[0];
        assert_eq!(bbox.len(), 6);

        // Summaries should contain merged metadata
        let summaries = collection.summaries.as_ref().unwrap();
        // Check proj:epsg
        let epsg_codes = summaries["proj:epsg"].as_array().unwrap();
        assert!(epsg_codes.iter().any(|c| c == 7415));
    }

    #[test]
    fn test_e2e_collection_serialization() {
        let path = test_data_path("delft.city.json");
        let reader = CityJSONReader::new(&path).expect("Failed to create reader");
        let readers: Vec<Box<dyn CityModelMetadataReader>> = vec![Box::new(reader)];

        let collection = StacCollectionBuilder::new("test")
            .aggregate_cityjson_metadata(&readers)
            .expect("Failed to aggregate")
            .build()
            .expect("Failed to build");

        let json = serde_json::to_string_pretty(&collection).expect("Failed to serialize");
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("Failed to parse");

        assert_eq!(parsed["type"], "Collection");
        assert_eq!(parsed["stac_version"], "1.1.0");
    }

    #[test]
    fn test_e2e_collection_output_to_file() {
        let path = test_data_path("delft.city.json");
        let reader = CityJSONReader::new(&path).expect("Failed to create reader");
        let readers: Vec<Box<dyn CityModelMetadataReader>> = vec![Box::new(reader)];

        let collection = StacCollectionBuilder::new("test")
            .aggregate_cityjson_metadata(&readers)
            .expect("Failed to aggregate")
            .build()
            .expect("Failed to build");

        let temp = tempdir().expect("Failed to create temp dir");
        let output_path = temp.path().join("collection.json");

        let json = serde_json::to_string_pretty(&collection).expect("Failed to serialize");
        std::fs::write(&output_path, &json).expect("Failed to write file");

        assert!(output_path.exists());
        let content = std::fs::read_to_string(&output_path).expect("Failed to read");
        assert!(content.contains("Collection"));
    }
}

mod e2e_workflow_tests {
    use super::*;

    #[test]
    fn test_e2e_full_workflow_with_items_and_collection() {
        // Simulate the full workflow: process multiple files, create items and collection

        let files = vec![
            test_data_path("delft.city.json"),
            test_data_path("railway.city.json"),
        ];

        let temp = tempdir().expect("Failed to create temp dir");

        // Process each file and create items
        let mut readers: Vec<Box<dyn CityModelMetadataReader>> = Vec::new();
        let mut items = Vec::new();

        for file_path in &files {
            let reader = get_reader(file_path).expect("Failed to get reader");

            let item = StacItemBuilder::from_file(file_path, reader.as_ref(), None)
                .expect("Failed to create builder")
                .build()
                .expect("Failed to build item");

            items.push(item);

            // Create a new reader for collection aggregation
            let collection_reader =
                CityJSONReader::new(file_path).expect("Failed to create reader");
            readers.push(Box::new(collection_reader));
        }

        // Build collection
        let collection = StacCollectionBuilder::new("test-workflow")
            .title("Test Workflow Collection")
            .description("Collection from e2e test")
            .aggregate_cityjson_metadata(&readers)
            .expect("Failed to aggregate")
            .build()
            .expect("Failed to build collection");

        // Write items
        for (i, item) in items.iter().enumerate() {
            let item_path = temp.path().join(format!("item_{}.json", i));
            let json = serde_json::to_string_pretty(item).expect("Failed to serialize");
            std::fs::write(&item_path, json).expect("Failed to write");
            assert!(item_path.exists());
        }

        // Write collection
        let collection_path = temp.path().join("collection.json");
        let json = serde_json::to_string_pretty(&collection).expect("Failed to serialize");
        std::fs::write(&collection_path, json).expect("Failed to write");
        assert!(collection_path.exists());

        // Verify all outputs
        assert_eq!(items.len(), 2);
        assert!(!collection.extent.spatial.bbox.is_empty());
    }

    #[test]
    fn test_e2e_metadata_preservation() {
        // This test verifies that metadata from source files is correctly
        // preserved in the generated STAC outputs

        let path = test_data_path("delft.city.json");

        // Read source file directly to get original metadata
        let source_content = std::fs::read_to_string(&path).expect("Failed to read source");
        let source_json: serde_json::Value =
            serde_json::from_str(&source_content).expect("Failed to parse source");

        // Create STAC item
        let reader = get_reader(&path).expect("Failed to get reader");
        let item = StacItemBuilder::from_file(&path, reader.as_ref(), None)
            .expect("Failed to create builder")
            .build()
            .expect("Failed to build");

        // Verify version matches
        let source_version = source_json["version"].as_str().unwrap();
        let item_version = item.properties["city3d:version"].as_str().unwrap();
        assert_eq!(source_version, item_version);

        // Verify bbox is transformed to WGS84 (lon/lat coordinates)
        // The source data is in EPSG:7415 (RD New), so the bbox should now be
        // in WGS84 with longitude/latitude values reasonable for Delft, Netherlands
        let item_bbox = item.bbox.as_ref().unwrap();
        assert_eq!(item_bbox.len(), 6);

        // Delft is approximately at lon 4.3, lat 52.0
        let lon_min = item_bbox[0];
        let lat_min = item_bbox[1];
        let lon_max = item_bbox[3];
        let lat_max = item_bbox[4];

        assert!(
            lon_min > 3.0 && lon_min < 6.0,
            "lon_min={lon_min} should be ~4.x"
        );
        assert!(
            lat_min > 51.0 && lat_min < 53.0,
            "lat_min={lat_min} should be ~52.x"
        );
        assert!(
            lon_max > 3.0 && lon_max < 6.0,
            "lon_max={lon_max} should be ~4.x"
        );
        assert!(
            lat_max > 51.0 && lat_max < 53.0,
            "lat_max={lat_max} should be ~52.x"
        );
        assert!(lon_min <= lon_max, "lon_min should be <= lon_max");
        assert!(lat_min <= lat_max, "lat_min should be <= lat_max");

        // Z values should be preserved from original extent
        let source_extent = source_json["metadata"]["geographicalExtent"]
            .as_array()
            .unwrap();
        let source_zmin = source_extent[2].as_f64().unwrap();
        let source_zmax = source_extent[5].as_f64().unwrap();
        assert!(
            (item_bbox[2] - source_zmin).abs() < 0.001,
            "zmin should be preserved"
        );
        assert!(
            (item_bbox[5] - source_zmax).abs() < 0.001,
            "zmax should be preserved"
        );

        // Verify the native CRS is preserved in proj:epsg
        assert!(item.properties.contains_key("proj:epsg"));
    }
}

mod e2e_error_handling_tests {
    use super::*;

    #[test]
    fn test_e2e_nonexistent_file_error() {
        let path = Path::new("/nonexistent/path/data.city.json");
        let result = get_reader(path);
        assert!(result.is_err());
    }

    #[test]
    fn test_e2e_unsupported_format_error() {
        let temp = tempdir().expect("Failed to create temp dir");
        let path = temp.path().join("data.txt");
        std::fs::write(&path, "not a cityjson file").expect("Failed to write");

        let result = get_reader(&path);
        assert!(result.is_err());
        if let Err(e) = result {
            let err = e.to_string();
            assert!(err.contains("Unsupported"));
        }
    }

    #[test]
    fn test_e2e_invalid_json_error() {
        let temp = tempdir().expect("Failed to create temp dir");
        let path = temp.path().join("invalid.json");
        std::fs::write(&path, "{ invalid json }").expect("Failed to write");

        let reader = CityJSONReader::new(&path);
        assert!(reader.is_ok()); // Reader creation succeeds

        // But metadata extraction should fail
        let r = reader.unwrap();
        let result = r.version();
        assert!(result.is_err());
    }
}

mod e2e_zip_file_tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;
    use zip::write::SimpleFileOptions;
    use zip::ZipWriter;

    /// Helper to create a ZIP file containing the delft.city.json test data
    fn create_zip_with_cityjson() -> NamedTempFile {
        // Read the source CityJSON file
        let source_path = test_data_path("delft.city.json");
        let cityjson_content =
            std::fs::read_to_string(&source_path).expect("Failed to read delft.city.json");

        // Create a ZIP file
        let temp_zip = NamedTempFile::with_suffix(".zip").expect("Failed to create temp ZIP file");
        let mut zip = ZipWriter::new(temp_zip.as_file());

        let options =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);

        zip.start_file("delft.city.json", options)
            .expect("Failed to start ZIP entry");
        zip.write_all(cityjson_content.as_bytes())
            .expect("Failed to write to ZIP");
        zip.finish().expect("Failed to finish ZIP");

        temp_zip
    }

    #[test]
    fn test_e2e_zip_reader_factory() {
        // Create a ZIP file with CityJSON content
        let temp_zip = create_zip_with_cityjson();

        // Test that get_reader() returns a ZipReader for .zip files
        let reader = get_reader(temp_zip.path()).expect("Failed to get reader for ZIP file");

        // Verify the encoding is CityJSON (from inner file)
        assert_eq!(reader.encoding(), "CityJSON");
    }

    #[test]
    fn test_e2e_zip_metadata_extraction() {
        let temp_zip = create_zip_with_cityjson();
        let reader = get_reader(temp_zip.path()).expect("Failed to get reader");

        // Verify metadata is extracted from the inner CityJSON file
        let version = reader.version().expect("Failed to get version");
        assert_eq!(version, "2.0");

        let bbox = reader.bbox().expect("Failed to get bbox");
        // These values match the metadata.geographicalExtent in delft.city.json
        assert_eq!(bbox.xmin, 84927.558);
        assert_eq!(bbox.xmax, 85527.591);

        let crs = reader.crs().expect("Failed to get CRS");
        assert_eq!(crs.to_stac_epsg(), Some(7415));
    }

    #[test]
    fn test_e2e_zip_to_stac_item() {
        let temp_zip = create_zip_with_cityjson();
        let reader = get_reader(temp_zip.path()).expect("Failed to get reader");

        // Build STAC item from ZIP file
        let item = StacItemBuilder::from_file(temp_zip.path(), reader.as_ref(), None)
            .expect("Failed to create item builder")
            .build()
            .expect("Failed to build item");

        // Validate basic STAC structure
        assert_eq!(item.stac_version, "1.1.0");
        assert_eq!(item.item_type, "Feature");

        // Validate metadata from inner CityJSON
        assert_eq!(item.properties["city3d:version"], "2.0");
        assert_eq!(item.properties["proj:epsg"], 7415);

        // Validate asset has application/zip media type
        assert!(item.assets.contains_key("data"));
        let data_asset = &item.assets["data"];
        assert_eq!(data_asset.media_type, Some("application/zip".to_string()));
    }

    #[test]
    fn test_e2e_zip_with_base_url() {
        let temp_zip = create_zip_with_cityjson();
        let reader = get_reader(temp_zip.path()).expect("Failed to get reader");

        let item = StacItemBuilder::from_file(
            temp_zip.path(),
            reader.as_ref(),
            Some("https://example.com/data"),
        )
        .expect("Failed to create item builder")
        .build()
        .expect("Failed to build item");

        // Validate asset href includes base URL
        assert!(item.assets.contains_key("data"));
        let data_asset = &item.assets["data"];
        assert!(data_asset.href.starts_with("https://example.com/data/"));
        assert!(data_asset.href.ends_with(".zip"));
    }

    #[test]
    fn test_e2e_zip_empty_archive_error() {
        // Create an empty ZIP file
        let temp_zip = NamedTempFile::with_suffix(".zip").expect("Failed to create temp ZIP");
        let zip = ZipWriter::new(temp_zip.as_file());
        zip.finish().expect("Failed to finish ZIP");

        // Should fail with "No CityJSON/CityGML files found"
        let result = get_reader(temp_zip.path());
        assert!(result.is_err());
        if let Err(e) = result {
            let err = e.to_string();
            assert!(err.contains("No CityJSON/CityGML files found"));
        }
    }
}
