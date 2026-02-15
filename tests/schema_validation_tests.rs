//! End-to-end schema validation tests
//!
//! These tests generate STAC Items and Collections from real CityJSON test data,
//! then validate the JSON output against:
//! 1. Core STAC Item/Collection JSON Schemas (self-contained, based on STAC v1.0.0)
//! 2. STAC 3D City Models Extension JSON Schema (based on stac-cityjson-extension)
//!
//! This ensures generated output is fully schema-compliant.

use city3d_stac::reader::{get_reader, CityJSONReader, CityJSONSeqReader, CityModelMetadataReader};
use city3d_stac::stac::{StacCollectionBuilder, StacItemBuilder};
use serde_json::Value;
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Schema loading helpers
// ---------------------------------------------------------------------------

fn load_stac_item_schema() -> Value {
    serde_json::from_str(include_str!("schemas/stac-item.json"))
        .expect("Failed to parse STAC Item schema")
}

fn load_stac_collection_schema() -> Value {
    serde_json::from_str(include_str!("schemas/stac-collection.json"))
        .expect("Failed to parse STAC Collection schema")
}

fn load_cityjson_extension_schema() -> Value {
    serde_json::from_str(include_str!("schemas/cityjson-extension.json"))
        .expect("Failed to parse CityJSON Extension schema")
}

// ---------------------------------------------------------------------------
// Validation helpers
// ---------------------------------------------------------------------------

/// Validate a JSON value against a schema, returning detailed error messages on failure
fn validate_against_schema(instance: &Value, schema: &Value, schema_name: &str) {
    let validator = jsonschema::validator_for(schema)
        .unwrap_or_else(|e| panic!("Invalid schema '{schema_name}': {e}"));

    let errors: Vec<String> = validator
        .iter_errors(instance)
        .map(|e| format!("  - {} (at {})", e, e.instance_path))
        .collect();

    if !errors.is_empty() {
        let json_preview = serde_json::to_string_pretty(instance)
            .unwrap_or_else(|_| "Failed to serialize".to_string());

        panic!(
            "\nSchema validation failed for '{schema_name}':\n{}\n\nJSON (first 2000 chars):\n{}",
            errors.join("\n"),
            &json_preview[..json_preview.len().min(2000)]
        );
    }
}

// ---------------------------------------------------------------------------
// Test data helpers
// ---------------------------------------------------------------------------

fn test_data_path(filename: &str) -> PathBuf {
    PathBuf::from("tests/data").join(filename)
}

/// Build a STAC Item from a file and return it as serde_json::Value
fn build_item_from_file(filename: &str) -> Value {
    let path = test_data_path(filename);
    let reader = get_reader(&path).expect(&format!("Failed to create reader for {filename}"));
    let builder = StacItemBuilder::from_file(&path, reader.as_ref(), None)
        .expect(&format!("Failed to build item for {filename}"));
    let item = builder.build().expect("Failed to build STAC Item");
    serde_json::to_value(item).expect("Failed to serialize STAC Item")
}

/// Build a STAC Collection from multiple readers and return as serde_json::Value
fn build_collection_from_readers(readers: &[Box<dyn CityModelMetadataReader>], id: &str) -> Value {
    let collection = StacCollectionBuilder::new(id)
        .license("proprietary")
        .description("Test collection for schema validation")
        .temporal_extent(Some(chrono::Utc::now()), None)
        .aggregate_cityjson_metadata(readers)
        .expect("Failed to aggregate metadata")
        .self_link("./collection.json")
        .build()
        .expect("Failed to build STAC Collection");

    serde_json::to_value(collection).expect("Failed to serialize STAC Collection")
}

// ===========================================================================
// STAC Item schema validation tests
// ===========================================================================

mod item_core_schema_tests {
    use super::*;

    #[test]
    fn test_delft_cityjson_item_validates_against_stac_item_schema() {
        let item = build_item_from_file("delft.city.json");
        let schema = load_stac_item_schema();
        validate_against_schema(&item, &schema, "STAC Item (delft.city.json)");
    }

    #[test]
    fn test_railway_cityjson_item_validates_against_stac_item_schema() {
        let item = build_item_from_file("railway.city.json");
        let schema = load_stac_item_schema();
        validate_against_schema(&item, &schema, "STAC Item (railway.city.json)");
    }

    #[test]
    fn test_delft_cjseq_item_validates_against_stac_item_schema() {
        let item = build_item_from_file("delft.city.jsonl");
        let schema = load_stac_item_schema();
        validate_against_schema(&item, &schema, "STAC Item (delft.city.jsonl)");
    }

    #[test]
    fn test_railway_cjseq_item_validates_against_stac_item_schema() {
        let item = build_item_from_file("railway.city.jsonl");
        let schema = load_stac_item_schema();
        validate_against_schema(&item, &schema, "STAC Item (railway.city.jsonl)");
    }
}

// ===========================================================================
// STAC 3D City Models Extension schema validation tests (Items)
// ===========================================================================

mod item_extension_schema_tests {
    use super::*;

    #[test]
    fn test_delft_cityjson_item_validates_against_extension_schema() {
        let item = build_item_from_file("delft.city.json");
        let schema = load_cityjson_extension_schema();
        validate_against_schema(&item, &schema, "CityJSON Extension (delft.city.json)");
    }

    #[test]
    fn test_railway_cityjson_item_validates_against_extension_schema() {
        let item = build_item_from_file("railway.city.json");
        let schema = load_cityjson_extension_schema();
        validate_against_schema(&item, &schema, "CityJSON Extension (railway.city.json)");
    }

    #[test]
    fn test_delft_cjseq_item_validates_against_extension_schema() {
        let item = build_item_from_file("delft.city.jsonl");
        let schema = load_cityjson_extension_schema();
        validate_against_schema(&item, &schema, "CityJSON Extension (delft.city.jsonl)");
    }

    #[test]
    fn test_railway_cjseq_item_validates_against_extension_schema() {
        let item = build_item_from_file("railway.city.jsonl");
        let schema = load_cityjson_extension_schema();
        validate_against_schema(&item, &schema, "CityJSON Extension (railway.city.jsonl)");
    }
}

// ===========================================================================
// STAC Item property content validation tests
// ===========================================================================

mod item_property_tests {
    use super::*;

    #[test]
    fn test_cityjson_item_has_required_extension_url() {
        let item = build_item_from_file("railway.city.json");
        let extensions = item["stac_extensions"].as_array().unwrap();
        assert!(
            extensions.iter().any(|e| e.as_str()
                == Some("https://stac-extensions.github.io/3d-city-models/v0.1.0/schema.json")),
            "Missing 3D City Models extension URL in stac_extensions"
        );
    }

    #[test]
    fn test_cityjson_item_has_projection_extension() {
        let item = build_item_from_file("delft.city.json");
        let extensions = item["stac_extensions"].as_array().unwrap();
        assert!(
            extensions
                .iter()
                .any(|e| e.as_str().is_some_and(|s| s.contains("projection"))),
            "Missing Projection extension URL in stac_extensions"
        );
    }

    #[test]
    fn test_cityjson_item_encoding_is_correct() {
        let item = build_item_from_file("delft.city.json");
        assert_eq!(
            item["properties"]["city3d:encoding"].as_str().unwrap(),
            "CityJSON"
        );
    }

    #[test]
    fn test_cjseq_item_encoding_is_correct() {
        let item = build_item_from_file("delft.city.jsonl");
        assert_eq!(
            item["properties"]["city3d:encoding"].as_str().unwrap(),
            "CityJSONSeq"
        );
    }

    #[test]
    fn test_cityjson_item_version_is_string() {
        let item = build_item_from_file("railway.city.json");
        assert!(
            item["properties"]["city3d:version"].is_string(),
            "city3d:version should be a string"
        );
    }

    #[test]
    fn test_cityjson_item_city_objects_is_integer() {
        let item = build_item_from_file("railway.city.json");
        assert!(
            item["properties"]["city3d:city_objects"].is_u64()
                || item["properties"]["city3d:city_objects"].is_i64(),
            "city3d:city_objects should be an integer for Items"
        );
    }

    #[test]
    fn test_railway_item_has_lods() {
        let item = build_item_from_file("railway.city.json");
        let lods = item["properties"]["city3d:lods"].as_array();
        assert!(lods.is_some(), "railway.city.json should have city3d:lods");
        assert!(
            !lods.unwrap().is_empty(),
            "city3d:lods should not be empty for railway"
        );
    }

    #[test]
    fn test_railway_item_has_co_types() {
        let item = build_item_from_file("railway.city.json");
        let types = item["properties"]["city3d:co_types"].as_array();
        assert!(
            types.is_some(),
            "railway.city.json should have city3d:co_types"
        );
        assert!(
            !types.unwrap().is_empty(),
            "city3d:co_types should not be empty for railway"
        );
    }

    #[test]
    fn test_railway_item_has_attributes() {
        let item = build_item_from_file("railway.city.json");
        let attrs = item["properties"]["city3d:attributes"].as_array();
        assert!(
            attrs.is_some(),
            "railway.city.json should have city3d:attributes"
        );

        // Each attribute should have name and type
        for attr in attrs.unwrap() {
            assert!(
                attr["name"].is_string(),
                "Attribute should have a 'name' field"
            );
            assert!(
                attr["type"].is_string(),
                "Attribute should have a 'type' field"
            );
        }
    }

    #[test]
    fn test_item_has_bbox_and_geometry() {
        let item = build_item_from_file("railway.city.json");

        // bbox should be a 6-element array (3D)
        let bbox = item["bbox"].as_array().unwrap();
        assert_eq!(bbox.len(), 6, "bbox should have 6 elements for 3D");

        // geometry should be a Polygon
        assert_eq!(
            item["geometry"]["type"].as_str().unwrap(),
            "Polygon",
            "geometry should be a Polygon"
        );
    }

    #[test]
    fn test_item_has_proj_epsg() {
        let item = build_item_from_file("delft.city.json");
        let epsg = item["properties"]["proj:epsg"].as_u64();
        assert!(epsg.is_some(), "Item should have proj:epsg property");
        assert_eq!(epsg.unwrap(), 7415);
    }

    #[test]
    fn test_item_has_data_asset() {
        let item = build_item_from_file("railway.city.json");
        let data_asset = &item["assets"]["data"];
        assert!(data_asset.is_object(), "Item should have a 'data' asset");
        assert!(data_asset["href"].is_string(), "Asset should have an href");
        assert!(
            data_asset["type"].is_string(),
            "Asset should have a media type"
        );
        assert!(data_asset["roles"].is_array(), "Asset should have roles");
    }

    #[test]
    fn test_item_has_datetime() {
        let item = build_item_from_file("delft.city.json");
        assert!(
            item["properties"]["datetime"].is_string(),
            "Item should have a datetime property"
        );
    }

    #[test]
    fn test_cjseq_item_city_objects_count_matches() {
        // CityJSONSeq streams and counts all city objects
        let item = build_item_from_file("railway.city.jsonl");
        let count = item["properties"]["city3d:city_objects"].as_u64();
        assert!(
            count.is_some(),
            "CityJSONSeq item should have city_objects count"
        );
        assert!(
            count.unwrap() > 0,
            "railway.city.jsonl should have city objects"
        );
    }
}

// ===========================================================================
// STAC Item from_content validation tests (remote reader path)
// ===========================================================================

mod from_content_schema_tests {
    use super::*;

    #[test]
    fn test_cityjson_from_content_validates_against_schemas() {
        let content = std::fs::read_to_string(test_data_path("railway.city.json"))
            .expect("Failed to read railway.city.json");

        let reader =
            CityJSONReader::from_content(&content, PathBuf::from("remote_railway.city.json"))
                .expect("Failed to create reader from content");

        let builder = StacItemBuilder::new("remote-railway")
            .cityjson_metadata(&reader)
            .expect("Failed to add metadata");

        let builder = if let Ok(bbox) = reader.bbox() {
            builder.bbox(bbox).geometry_from_bbox()
        } else {
            builder
        };

        let builder = builder.data_asset("remote_railway.city.json", "application/json");
        let item = builder.build().expect("Failed to build item");
        let item_json = serde_json::to_value(item).unwrap();

        // Validate against both schemas
        validate_against_schema(
            &item_json,
            &load_stac_item_schema(),
            "STAC Item (from_content)",
        );
        validate_against_schema(
            &item_json,
            &load_cityjson_extension_schema(),
            "CityJSON Extension (from_content)",
        );
    }

    #[test]
    fn test_cjseq_from_content_validates_against_schemas() {
        let content = std::fs::read_to_string(test_data_path("railway.city.jsonl"))
            .expect("Failed to read railway.city.jsonl");

        let reader =
            CityJSONSeqReader::from_content(&content, PathBuf::from("remote_railway.city.jsonl"))
                .expect("Failed to create reader from content");

        let builder = StacItemBuilder::new("remote-railway-cjseq")
            .cityjson_metadata(&reader)
            .expect("Failed to add metadata");

        let builder = if let Ok(bbox) = reader.bbox() {
            builder.bbox(bbox).geometry_from_bbox()
        } else {
            builder
        };

        let builder = builder.data_asset("remote_railway.city.jsonl", "application/json-seq");
        let item = builder.build().expect("Failed to build item");
        let item_json = serde_json::to_value(item).unwrap();

        // Validate against both schemas
        validate_against_schema(
            &item_json,
            &load_stac_item_schema(),
            "STAC Item (cjseq from_content)",
        );
        validate_against_schema(
            &item_json,
            &load_cityjson_extension_schema(),
            "CityJSON Extension (cjseq from_content)",
        );
    }
}

// ===========================================================================
// STAC Collection schema validation tests
// ===========================================================================

mod collection_core_schema_tests {
    use super::*;

    fn build_test_collection() -> Value {
        let files = vec![
            "delft.city.json",
            "railway.city.json",
            "delft.city.jsonl",
            "railway.city.jsonl",
        ];

        let readers: Vec<Box<dyn CityModelMetadataReader>> = files
            .iter()
            .map(|f| get_reader(&test_data_path(f)).expect(&format!("Failed to read {f}")))
            .collect();

        build_collection_from_readers(&readers, "test-collection")
    }

    #[test]
    fn test_collection_validates_against_stac_collection_schema() {
        let collection = build_test_collection();
        let schema = load_stac_collection_schema();
        validate_against_schema(&collection, &schema, "STAC Collection (core)");
    }

    #[test]
    fn test_collection_validates_against_extension_schema() {
        let collection = build_test_collection();
        let schema = load_cityjson_extension_schema();
        validate_against_schema(&collection, &schema, "CityJSON Extension (collection)");
    }

    #[test]
    fn test_single_format_collection_validates() {
        // Collection from only CityJSON files
        let readers: Vec<Box<dyn CityModelMetadataReader>> = vec![
            get_reader(&test_data_path("delft.city.json")).unwrap(),
            get_reader(&test_data_path("railway.city.json")).unwrap(),
        ];

        let collection = build_collection_from_readers(&readers, "cityjson-only");

        validate_against_schema(
            &collection,
            &load_stac_collection_schema(),
            "STAC Collection (CityJSON only)",
        );
        validate_against_schema(
            &collection,
            &load_cityjson_extension_schema(),
            "CityJSON Extension (CityJSON only collection)",
        );
    }

    #[test]
    fn test_cjseq_only_collection_validates() {
        // Collection from only CityJSONSeq files
        let readers: Vec<Box<dyn CityModelMetadataReader>> = vec![
            get_reader(&test_data_path("delft.city.jsonl")).unwrap(),
            get_reader(&test_data_path("railway.city.jsonl")).unwrap(),
        ];

        let collection = build_collection_from_readers(&readers, "cjseq-only");

        validate_against_schema(
            &collection,
            &load_stac_collection_schema(),
            "STAC Collection (CityJSONSeq only)",
        );
        validate_against_schema(
            &collection,
            &load_cityjson_extension_schema(),
            "CityJSON Extension (CityJSONSeq only collection)",
        );
    }
}

// ===========================================================================
// STAC Collection property content validation tests
// ===========================================================================

mod collection_property_tests {
    use super::*;

    fn build_mixed_collection() -> Value {
        let readers: Vec<Box<dyn CityModelMetadataReader>> = vec![
            get_reader(&test_data_path("delft.city.json")).unwrap(),
            get_reader(&test_data_path("railway.city.json")).unwrap(),
            get_reader(&test_data_path("delft.city.jsonl")).unwrap(),
            get_reader(&test_data_path("railway.city.jsonl")).unwrap(),
        ];

        build_collection_from_readers(&readers, "mixed-collection")
    }

    #[test]
    fn test_collection_has_required_extension_url() {
        let collection = build_mixed_collection();
        let extensions = collection["stac_extensions"].as_array().unwrap();
        assert!(
            extensions.iter().any(|e| e.as_str()
                == Some("https://stac-extensions.github.io/3d-city-models/v0.1.0/schema.json")),
            "Missing 3D City Models extension URL in stac_extensions"
        );
    }

    #[test]
    fn test_collection_type_is_collection() {
        let collection = build_mixed_collection();
        assert_eq!(collection["type"].as_str().unwrap(), "Collection");
    }

    #[test]
    fn test_collection_has_spatial_extent() {
        let collection = build_mixed_collection();
        let bbox = collection["extent"]["spatial"]["bbox"]
            .as_array()
            .expect("Missing spatial.bbox");
        assert!(!bbox.is_empty(), "spatial.bbox should not be empty");

        // Each bbox should be a 4 or 6 element array
        for b in bbox {
            let arr = b.as_array().unwrap();
            assert!(
                arr.len() == 4 || arr.len() == 6,
                "Each bbox should have 4 or 6 elements, got {}",
                arr.len()
            );
        }
    }

    #[test]
    fn test_collection_has_temporal_extent() {
        let collection = build_mixed_collection();
        let intervals = collection["extent"]["temporal"]["interval"]
            .as_array()
            .expect("Missing temporal.interval");
        assert!(
            !intervals.is_empty(),
            "temporal.interval should not be empty"
        );

        // Each interval should be a 2-element array
        for interval in intervals {
            let arr = interval.as_array().unwrap();
            assert_eq!(arr.len(), 2, "Each interval should have 2 elements");
        }
    }

    #[test]
    fn test_collection_summaries_encoding() {
        let collection = build_mixed_collection();
        let summaries = &collection["summaries"];

        let encodings = summaries["city3d:encoding"]
            .as_array()
            .expect("Missing city3d:encoding in summaries");

        // Mixed collection should contain both CityJSON and CityJSONSeq
        let encoding_strings: Vec<&str> = encodings.iter().filter_map(|v| v.as_str()).collect();
        assert!(
            encoding_strings.contains(&"CityJSON"),
            "Should contain CityJSON encoding"
        );
        assert!(
            encoding_strings.contains(&"CityJSONSeq"),
            "Should contain CityJSONSeq encoding"
        );
    }

    #[test]
    fn test_collection_summaries_city_objects_statistics() {
        let collection = build_mixed_collection();
        let summaries = &collection["summaries"];

        let city_objects = &summaries["city3d:city_objects"];
        assert!(
            city_objects.is_object(),
            "city3d:city_objects should be a statistics object in collections"
        );

        // Should have min, max, total
        assert!(
            city_objects["min"].is_u64() || city_objects["min"].is_i64(),
            "city_objects.min should be integer"
        );
        assert!(
            city_objects["max"].is_u64() || city_objects["max"].is_i64(),
            "city_objects.max should be integer"
        );
        assert!(
            city_objects["total"].is_u64() || city_objects["total"].is_i64(),
            "city_objects.total should be integer"
        );

        // total >= max >= min
        let min = city_objects["min"].as_u64().unwrap();
        let max = city_objects["max"].as_u64().unwrap();
        let total = city_objects["total"].as_u64().unwrap();
        assert!(max >= min, "max ({max}) should be >= min ({min})");
        assert!(total >= max, "total ({total}) should be >= max ({max})");
    }

    #[test]
    fn test_collection_summaries_lods() {
        let collection = build_mixed_collection();
        let summaries = &collection["summaries"];

        let lods = summaries["city3d:lods"].as_array();
        assert!(lods.is_some(), "Collection should have city3d:lods summary");

        // All LODs should be strings
        for lod in lods.unwrap() {
            assert!(lod.is_string(), "Each LOD should be a string, got: {lod}");
        }
    }

    #[test]
    fn test_collection_summaries_co_types() {
        let collection = build_mixed_collection();
        let summaries = &collection["summaries"];

        let types = summaries["city3d:co_types"].as_array();
        assert!(
            types.is_some(),
            "Collection should have city3d:co_types summary"
        );

        // All types should be strings
        for t in types.unwrap() {
            assert!(t.is_string(), "Each co_type should be a string, got: {t}");
        }
    }

    #[test]
    fn test_collection_has_proj_epsg_summary() {
        let collection = build_mixed_collection();
        let summaries = &collection["summaries"];

        let epsg = summaries["proj:epsg"].as_array();
        assert!(epsg.is_some(), "Collection should have proj:epsg summary");
        assert!(!epsg.unwrap().is_empty(), "proj:epsg should not be empty");
    }
}

// ===========================================================================
// Collection from aggregated items (update-collection path)
// ===========================================================================

mod aggregate_collection_schema_tests {
    use super::*;
    use city3d_stac::stac::StacItem;

    #[test]
    fn test_aggregate_collection_validates_against_schemas() {
        // First, generate STAC Items from all test files
        let files = vec![
            "delft.city.json",
            "railway.city.json",
            "delft.city.jsonl",
            "railway.city.jsonl",
        ];

        let items: Vec<StacItem> = files
            .iter()
            .map(|f| {
                let path = test_data_path(f);
                let reader = get_reader(&path).expect(&format!("Failed to read {f}"));
                let builder = StacItemBuilder::from_file(&path, reader.as_ref(), None)
                    .expect(&format!("Failed to build item for {f}"));
                builder.build().expect("Failed to build STAC Item")
            })
            .collect();

        // Aggregate into a collection using update-collection path
        let mut collection_builder = StacCollectionBuilder::new("aggregated-collection")
            .license("proprietary")
            .description("Aggregated test collection")
            .temporal_extent(Some(chrono::Utc::now()), None)
            .aggregate_from_items(&items)
            .expect("Failed to aggregate from items");

        // Add item links
        for item in &items {
            collection_builder =
                collection_builder.item_link(format!("./{}.json", item.id), Some(item.id.clone()));
        }
        collection_builder = collection_builder.self_link("./collection.json");

        let collection = collection_builder
            .build()
            .expect("Failed to build collection");
        let collection_json = serde_json::to_value(collection).unwrap();

        // Validate against schemas
        validate_against_schema(
            &collection_json,
            &load_stac_collection_schema(),
            "STAC Collection (aggregated)",
        );
        validate_against_schema(
            &collection_json,
            &load_cityjson_extension_schema(),
            "CityJSON Extension (aggregated collection)",
        );
    }

    #[test]
    fn test_aggregate_collection_preserves_encoding_info() {
        let files = vec!["delft.city.json", "delft.city.jsonl"];

        let items: Vec<StacItem> = files
            .iter()
            .map(|f| {
                let path = test_data_path(f);
                let reader = get_reader(&path).unwrap();
                StacItemBuilder::from_file(&path, reader.as_ref(), None)
                    .unwrap()
                    .build()
                    .unwrap()
            })
            .collect();

        let collection = StacCollectionBuilder::new("encoding-test")
            .license("proprietary")
            .description("Test encoding preservation")
            .temporal_extent(Some(chrono::Utc::now()), None)
            .aggregate_from_items(&items)
            .expect("Failed to aggregate")
            .self_link("./collection.json")
            .build()
            .expect("Failed to build");

        let json = serde_json::to_value(collection).unwrap();
        let encodings = json["summaries"]["city3d:encoding"]
            .as_array()
            .expect("Missing encodings");

        let encoding_strings: Vec<&str> = encodings.iter().filter_map(|v| v.as_str()).collect();
        assert!(
            encoding_strings.contains(&"CityJSON"),
            "Should preserve CityJSON encoding"
        );
        assert!(
            encoding_strings.contains(&"CityJSONSeq"),
            "Should preserve CityJSONSeq encoding"
        );
    }
}

// ===========================================================================
// Cross-format consistency tests
// ===========================================================================

mod cross_format_consistency_tests {
    use super::*;

    #[test]
    fn test_delft_json_and_jsonl_produce_same_metadata() {
        let json_item = build_item_from_file("delft.city.json");
        let jsonl_item = build_item_from_file("delft.city.jsonl");

        // Both should have the same CRS
        assert_eq!(
            json_item["properties"]["proj:epsg"], jsonl_item["properties"]["proj:epsg"],
            "CRS should match between CityJSON and CityJSONSeq for same dataset"
        );

        // Both should have the same version
        assert_eq!(
            json_item["properties"]["city3d:version"], jsonl_item["properties"]["city3d:version"],
            "Version should match between formats"
        );

        // Both should have the same bbox
        assert_eq!(
            json_item["bbox"], jsonl_item["bbox"],
            "BBox should match between CityJSON and CityJSONSeq for same dataset"
        );
    }

    #[test]
    fn test_railway_json_and_jsonl_produce_same_city_objects_count() {
        let json_item = build_item_from_file("railway.city.json");
        let jsonl_item = build_item_from_file("railway.city.jsonl");

        assert_eq!(
            json_item["properties"]["city3d:city_objects"],
            jsonl_item["properties"]["city3d:city_objects"],
            "City object count should match between CityJSON and CityJSONSeq"
        );
    }

    #[test]
    fn test_railway_json_and_jsonl_produce_same_co_types() {
        let json_item = build_item_from_file("railway.city.json");
        let jsonl_item = build_item_from_file("railway.city.jsonl");

        // co_types should contain the same elements (order may differ)
        let mut json_types: Vec<String> = json_item["properties"]["city3d:co_types"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect();
        json_types.sort();

        let mut jsonl_types: Vec<String> = jsonl_item["properties"]["city3d:co_types"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect();
        jsonl_types.sort();

        assert_eq!(
            json_types, jsonl_types,
            "City object types should match between formats"
        );
    }
}
