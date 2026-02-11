//! End-to-end integration tests

use cityjson_stac::reader::{get_reader, CityJSONReader, CityModelMetadataReader};
use cityjson_stac::stac::{StacCollectionBuilder, StacItemBuilder};
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
        assert_eq!(item.stac_version, "1.0.0");
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
        assert_eq!(item.properties["city3d:encoding"], "CityJSON");
        assert_eq!(item.properties["city3d:version"], "2.0");

        // Validate projection extension
        assert_eq!(item.properties["proj:epsg"], 7415);

        // Validate required STAC extensions
        assert!(item
            .stac_extensions
            .iter()
            .any(|e| e.contains("3d-city-models")));
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
        assert_eq!(deserialized["stac_version"], "1.0.0");
        assert_eq!(deserialized["type"], "Feature");
        assert!(deserialized["properties"]["city3d:encoding"].is_string());
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
        assert_eq!(parsed["stac_version"], "1.0.0");
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
        assert_eq!(collection.stac_version, "1.0.0");
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
        let encodings = summaries["city3d:encoding"].as_array().unwrap();
        assert!(encodings.iter().any(|e| e == "CityJSON"));
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
        assert_eq!(parsed["stac_version"], "1.0.0");
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

        // Verify bbox matches geographicalExtent
        let source_extent = source_json["metadata"]["geographicalExtent"]
            .as_array()
            .unwrap();
        let item_bbox = item.bbox.as_ref().unwrap();

        for i in 0..6 {
            let source_val = source_extent[i].as_f64().unwrap();
            let item_val = item_bbox[i];
            assert!(
                (source_val - item_val).abs() < 0.001,
                "bbox[{}] mismatch: source={}, item={}",
                i,
                source_val,
                item_val
            );
        }
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
