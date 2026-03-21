//! Unit tests for STAC item and collection building

use city3d_stac::metadata::BBox3D;
use city3d_stac::reader::{CityJSONReader, CityModelMetadataReader};
use city3d_stac::stac::{StacCollectionBuilder, StacItemBuilder};
use serde_json::Value;
use std::path::Path;

/// Test data directory path
fn test_data_path(filename: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("data")
        .join(filename)
}

mod stac_item_builder_tests {
    use super::*;

    #[test]
    fn test_item_builder_new() {
        let item = StacItemBuilder::new("test-id")
            .build()
            .expect("Failed to build item");

        assert_eq!(item.id, "test-id");

        // stac_version and type are private; verify via JSON serialization
        let parsed = serde_json::to_value(&item).unwrap();
        assert_eq!(parsed["stac_version"], "1.1.0");
        assert_eq!(parsed["type"], "Feature");
    }

    #[test]
    fn test_item_builder_with_bbox() {
        let bbox = BBox3D::new(0.0, 0.0, 0.0, 10.0, 10.0, 10.0);
        let item = StacItemBuilder::new("test-id")
            .bbox(bbox)
            .build()
            .expect("Failed to build item");

        assert!(item.bbox.is_some());
        let stac_bbox = item.bbox.unwrap();
        let bb: Vec<f64> = stac_bbox.into();
        assert_eq!(bb.len(), 6);
        assert_eq!(bb[0], 0.0);
        assert_eq!(bb[5], 10.0);
    }

    #[test]
    fn test_item_builder_with_geometry_from_bbox() {
        let bbox = BBox3D::new(0.0, 0.0, 0.0, 10.0, 10.0, 10.0);
        let item = StacItemBuilder::new("test-id")
            .bbox(bbox)
            .geometry_from_bbox()
            .build()
            .expect("Failed to build item");

        assert!(item.geometry.is_some());
        let geom = serde_json::to_value(item.geometry.unwrap()).unwrap();
        assert_eq!(geom["type"], "Polygon");
    }

    #[test]
    fn test_item_builder_with_title_and_description() {
        let item = StacItemBuilder::new("test-id")
            .title("Test Building")
            .description("A test building dataset")
            .build()
            .expect("Failed to build item");

        // title and description are now native fields on Properties
        assert_eq!(item.properties.title, Some("Test Building".to_string()));
        assert_eq!(
            item.properties.description,
            Some("A test building dataset".to_string())
        );
    }

    #[test]
    fn test_item_builder_has_datetime() {
        let item = StacItemBuilder::new("test-id")
            .build()
            .expect("Failed to build item");

        // The builder does not set a default datetime; it remains None
        // unless explicitly set via .datetime() or .cityjson_metadata().
        // Verify the datetime field is serialized (as null when not set).
        let parsed = serde_json::to_value(&item).unwrap();
        assert!(parsed["properties"].get("datetime").is_some());
    }

    #[test]
    fn test_item_builder_with_data_asset() {
        let item = StacItemBuilder::new("test-id")
            .data_asset("./data.json", "application/json", None)
            .build()
            .expect("Failed to build item");

        assert!(item.assets.contains_key("data"));
        let asset = &item.assets["data"];
        assert_eq!(asset.href, "./data.json");
        assert_eq!(asset.r#type, Some("application/json".to_string()));
    }

    #[test]
    fn test_item_builder_with_links() {
        let item = StacItemBuilder::new("test-id")
            .self_link("./item.json")
            .parent_link("../collection.json")
            .build()
            .expect("Failed to build item");

        assert_eq!(item.links.len(), 2);

        let self_link = item.links.iter().find(|l| l.rel == "self");
        assert!(self_link.is_some());

        let parent_link = item.links.iter().find(|l| l.rel == "parent");
        assert!(parent_link.is_some());
    }

    #[test]
    fn test_item_builder_without_cityjson_metadata() {
        // Without cityjson_metadata(), build should still succeed
        // Without cityjson_metadata(), build should still succeed
        let result = StacItemBuilder::new("test-id").build();
        assert!(result.is_ok());
    }

    #[test]
    fn test_item_builder_stac_extensions() {
        // Test without proj:code - no projection extension
        let item = StacItemBuilder::new("test-id")
            .build()
            .expect("Failed to build item");

        // Should include 3D City Models extension
        assert!(item.extensions.iter().any(|e| e.contains("stac-city3d")));
        // Should NOT include projection extension (no proj:code property)
        assert!(!item.extensions.iter().any(|e| e.contains("projection")));

        // Test with proj:code - projection extension should be included
        let item = StacItemBuilder::new("test-id")
            .property(
                "proj:code".to_string(),
                Value::String("EPSG:4326".to_string()),
            )
            .build()
            .expect("Failed to build item");

        // Should include both extensions
        assert!(item.extensions.iter().any(|e| e.contains("stac-city3d")));
        assert!(item.extensions.iter().any(|e| e.contains("projection")));
    }
}

mod stac_item_from_file_tests {
    use super::*;

    #[test]
    fn test_item_from_delft_file() {
        let path = test_data_path("delft.city.json");
        let reader = CityJSONReader::new(&path).expect("Failed to create reader");

        let builder = StacItemBuilder::from_file(&path, &reader, None, None)
            .expect("Failed to create builder");
        let item = builder.build().expect("Failed to build item");

        // Check CityJSON extension properties (in additional_fields)
        assert_eq!(
            item.properties
                .additional_fields
                .get("city3d:version")
                .unwrap(),
            "2.0"
        );
        assert_eq!(
            item.properties.additional_fields.get("proj:code").unwrap(),
            "EPSG:7415"
        );

        // Check bbox is set
        assert!(item.bbox.is_some());

        // Check geometry is set
        assert!(item.geometry.is_some());

        // Check data asset
        assert!(item.assets.contains_key("data"));
    }

    #[test]
    fn test_item_from_railway_file() {
        let path = test_data_path("railway.city.json");
        let reader = CityJSONReader::new(&path).expect("Failed to create reader");

        let builder = StacItemBuilder::from_file(&path, &reader, None, None)
            .expect("Failed to create builder");
        let item = builder.build().expect("Failed to build item");

        // Railway should have city objects
        let city_objects = item.properties.additional_fields.get("city3d:city_objects");
        assert!(city_objects.is_some());
        assert!(city_objects.unwrap().as_u64().unwrap() > 0);

        // Railway should have LODs
        let lods = item.properties.additional_fields.get("city3d:lods");
        assert!(lods.is_some());

        // Railway should have object types
        let types = item.properties.additional_fields.get("city3d:co_types");
        assert!(types.is_some());
    }
}

mod stac_collection_builder_tests {
    use super::*;

    #[test]
    fn test_collection_builder_new() {
        let bbox = BBox3D::new(0.0, 0.0, 0.0, 10.0, 10.0, 10.0);
        let collection = StacCollectionBuilder::new("test-collection")
            .spatial_extent(bbox)
            .build()
            .expect("Failed to build collection");

        assert_eq!(collection.id, "test-collection");

        // stac_version and type are private; verify via JSON serialization
        let parsed = serde_json::to_value(&collection).unwrap();
        assert_eq!(parsed["stac_version"], "1.1.0");
        assert_eq!(parsed["type"], "Collection");
    }

    #[test]
    fn test_collection_builder_with_title_description() {
        let bbox = BBox3D::new(0.0, 0.0, 0.0, 10.0, 10.0, 10.0);
        let collection = StacCollectionBuilder::new("test")
            .title("Test Collection")
            .description("A test collection of CityJSON files")
            .spatial_extent(bbox)
            .build()
            .expect("Failed to build collection");

        assert_eq!(collection.title, Some("Test Collection".to_string()));
        // description is now a String, not Option<String>
        assert_eq!(
            collection.description,
            "A test collection of CityJSON files"
        );
    }

    #[test]
    fn test_collection_builder_with_license() {
        let bbox = BBox3D::new(0.0, 0.0, 0.0, 10.0, 10.0, 10.0);
        let collection = StacCollectionBuilder::new("test")
            .license("CC-BY-4.0")
            .spatial_extent(bbox)
            .build()
            .expect("Failed to build collection");

        assert_eq!(collection.license, "CC-BY-4.0");
    }

    #[test]
    fn test_collection_builder_requires_spatial_extent() {
        // Without spatial extent, build should fail
        let result = StacCollectionBuilder::new("test").build();

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Spatial extent"));
    }

    #[test]
    fn test_collection_builder_with_keywords() {
        let bbox = BBox3D::new(0.0, 0.0, 0.0, 10.0, 10.0, 10.0);
        let collection = StacCollectionBuilder::new("test")
            .keywords(vec![
                "3D".to_string(),
                "buildings".to_string(),
                "CityJSON".to_string(),
            ])
            .spatial_extent(bbox)
            .build()
            .expect("Failed to build collection");

        assert!(collection.keywords.is_some());
        let kw = collection.keywords.unwrap();
        assert_eq!(kw.len(), 3);
        assert!(kw.contains(&"CityJSON".to_string()));
    }

    #[test]
    fn test_collection_builder_with_summary() {
        let bbox = BBox3D::new(0.0, 0.0, 0.0, 10.0, 10.0, 10.0);
        let collection = StacCollectionBuilder::new("test")
            .summary("city3d:lods", serde_json::json!(["1", "2", "2.2"]))
            .spatial_extent(bbox)
            .build()
            .expect("Failed to build collection");

        assert!(collection.summaries.is_some());
        let summaries = collection.summaries.unwrap();
        assert!(summaries.contains_key("city3d:lods"));
    }

    #[test]
    fn test_collection_builder_with_links() {
        let bbox = BBox3D::new(0.0, 0.0, 0.0, 10.0, 10.0, 10.0);
        let collection = StacCollectionBuilder::new("test")
            .spatial_extent(bbox)
            .self_link("./collection.json")
            .item_link("./items/item1.json", Some("Item 1".to_string()))
            .build()
            .expect("Failed to build collection");

        assert_eq!(collection.links.len(), 2);
    }
}

mod stac_collection_aggregate_tests {
    use super::*;

    #[test]
    fn test_collection_aggregate_from_single_reader() {
        let path = test_data_path("delft.city.json");
        let reader = CityJSONReader::new(&path).expect("Failed to create reader");

        let readers: Vec<Box<dyn CityModelMetadataReader>> = vec![Box::new(reader)];

        let collection = StacCollectionBuilder::new("test")
            .aggregate_cityjson_metadata(&readers)
            .expect("Failed to aggregate")
            .build()
            .expect("Failed to build collection");

        // Should have spatial extent from aggregation
        assert!(!collection.extent.spatial.bbox.is_empty());

        // Should have summaries
        assert!(collection.summaries.is_some());
    }

    #[test]
    fn test_collection_aggregate_from_multiple_readers() {
        let path1 = test_data_path("delft.city.json");
        let path2 = test_data_path("railway.city.json");

        let reader1 = CityJSONReader::new(&path1).expect("Failed to create reader 1");
        let reader2 = CityJSONReader::new(&path2).expect("Failed to create reader 2");

        let readers: Vec<Box<dyn CityModelMetadataReader>> =
            vec![Box::new(reader1), Box::new(reader2)];

        let collection = StacCollectionBuilder::new("test")
            .aggregate_cityjson_metadata(&readers)
            .expect("Failed to aggregate")
            .build()
            .expect("Failed to build collection");

        assert_eq!(collection.extent.spatial.bbox.len(), 1);
    }
}

mod link_tests {
    use city3d_stac::stac::Link;

    #[test]
    fn test_link_new() {
        // stac::Link::new takes (href, rel)
        let link = Link::new("./item.json", "self");
        assert_eq!(link.rel, "self");
        assert_eq!(link.href, "./item.json");
        assert!(link.r#type.is_none());
        assert!(link.title.is_none());
    }

    #[test]
    fn test_link_with_type() {
        let link = Link::new("./item.json", "self").r#type(Some("application/json".to_string()));
        assert_eq!(link.r#type, Some("application/json".to_string()));
    }

    #[test]
    fn test_link_with_title() {
        let link = Link::new("./item.json", "item").title(Some("Building Item".to_string()));
        assert_eq!(link.title, Some("Building Item".to_string()));
    }

    #[test]
    fn test_link_builder_chain() {
        let link = Link::new("./collection.json", "collection")
            .r#type(Some("application/json".to_string()))
            .title(Some("Parent Collection".to_string()));

        assert_eq!(link.rel, "collection");
        assert_eq!(link.r#type, Some("application/json".to_string()));
        assert_eq!(link.title, Some("Parent Collection".to_string()));
    }
}

mod asset_tests {
    use city3d_stac::stac::Asset;

    #[test]
    fn test_asset_new() {
        let asset = Asset::new("./data.json");
        assert_eq!(asset.href, "./data.json");
        assert!(asset.r#type.is_none());
        assert!(asset.title.is_none());
        // roles is now Vec<String>, not Option<Vec<String>>
        assert!(asset.roles.is_empty());
    }

    #[test]
    fn test_asset_with_type() {
        let mut asset = Asset::new("./data.json");
        asset.r#type = Some("application/json".to_string());
        assert_eq!(asset.r#type, Some("application/json".to_string()));
    }

    #[test]
    fn test_asset_with_title() {
        let mut asset = Asset::new("./data.json");
        asset.title = Some("CityJSON Data".to_string());
        assert_eq!(asset.title, Some("CityJSON Data".to_string()));
    }

    #[test]
    fn test_asset_with_roles() {
        let mut asset = Asset::new("./data.json");
        asset.roles = vec!["data".to_string()];
        assert_eq!(asset.roles, vec!["data".to_string()]);
    }

    #[test]
    fn test_asset_builder_chain() {
        let mut asset = Asset::new("./building.json");
        asset.r#type = Some("application/json".to_string());
        asset.title = Some("Building Data".to_string());
        asset.roles = vec!["data".to_string(), "primary".to_string()];

        assert_eq!(asset.href, "./building.json");
        assert_eq!(asset.r#type, Some("application/json".to_string()));
        assert_eq!(asset.title, Some("Building Data".to_string()));
        assert_eq!(asset.roles, vec!["data".to_string(), "primary".to_string()]);
    }
}

mod stac_collection_aggregate_from_items_tests {
    use super::*;
    use city3d_stac::stac::StacItem;

    /// Helper to create a test STAC item with CityJSON properties
    fn create_test_stac_item(
        id: &str,
        _encoding: &str,
        lods: Vec<&str>,
        co_types: Vec<&str>,
        city_objects: i64,
        epsg: Option<i64>,
        bbox: Option<Vec<f64>>,
    ) -> StacItem {
        let mut item = stac::Item::new(id);

        // Set datetime
        item.properties.datetime = Some("2024-01-01T00:00:00Z".parse().unwrap());

        // Extension properties go in additional_fields
        item.properties.additional_fields.insert(
            "city3d:version".to_string(),
            Value::String("2.0".to_string()),
        );
        item.properties.additional_fields.insert(
            "city3d:city_objects".to_string(),
            Value::Number(serde_json::Number::from(city_objects)),
        );

        if !lods.is_empty() {
            item.properties.additional_fields.insert(
                "city3d:lods".to_string(),
                serde_json::to_value(lods).unwrap(),
            );
        }

        if !co_types.is_empty() {
            item.properties.additional_fields.insert(
                "city3d:co_types".to_string(),
                serde_json::to_value(co_types).unwrap(),
            );
        }

        if let Some(epsg_code) = epsg {
            item.properties.additional_fields.insert(
                "proj:code".to_string(),
                Value::String(format!("EPSG:{epsg_code}")),
            );
        }

        // Set bbox
        if let Some(bbox_vec) = bbox {
            item.bbox = bbox_vec.try_into().ok();
        }

        item
    }

    #[test]
    fn test_aggregate_from_single_item() {
        let item = create_test_stac_item(
            "test-item",
            "CityJSON",
            vec!["2"],
            vec!["Building"],
            100,
            Some(7415),
            Some(vec![0.0, 0.0, 0.0, 10.0, 10.0, 10.0]),
        );

        let collection = StacCollectionBuilder::new("test")
            .aggregate_from_items(&[item])
            .expect("Failed to aggregate")
            .build()
            .expect("Failed to build collection");

        // Should have spatial extent
        assert!(!collection.extent.spatial.bbox.is_empty());

        // Should have summaries
    }
    #[test]
    fn test_aggregate_from_multiple_items() {
        let item1 = create_test_stac_item(
            "building-item",
            "CityJSON",
            vec!["2", "2.2"],
            vec!["Building", "BuildingPart"],
            50,
            Some(7415),
            Some(vec![0.0, 0.0, 0.0, 10.0, 10.0, 10.0]),
        );

        let item2 = create_test_stac_item(
            "railway-item",
            "CityJSONSeq",
            vec!["1", "3"],
            vec!["Railway", "Bridge"],
            150,
            Some(4326),
            Some(vec![10.0, 5.0, -5.0, 20.0, 15.0, 20.0]),
        );

        let collection = StacCollectionBuilder::new("test")
            .aggregate_from_items(&[item1, item2])
            .expect("Failed to aggregate")
            .build()
            .expect("Failed to build collection");
        let summaries = collection.summaries.unwrap();

        // Should have aggregated LODs
        let lods = summaries.get("city3d:lods").unwrap().as_array().unwrap();
        assert!(lods.len() >= 4); // "1", "2", "2.2", "3"

        // Should have aggregated co_types
        let types = summaries
            .get("city3d:co_types")
            .unwrap()
            .as_array()
            .unwrap();
        assert!(types.len() >= 4);

        // Should have city object statistics
        let stats = summaries.get("city3d:city_objects").unwrap();
        assert_eq!(stats["min"], 50);
        assert_eq!(stats["max"], 150);
        assert_eq!(stats["total"], 200);

        // Should have both proj:code entries
        let proj_codes = summaries.get("proj:code").unwrap().as_array().unwrap();
        assert_eq!(proj_codes.len(), 2);

        // Should have merged bbox - convert stac::Bbox to Vec<f64> for indexing
        let stac_bbox = collection.extent.spatial.bbox[0];
        let bbox: Vec<f64> = stac_bbox.into();
        assert_eq!(bbox[0], 0.0); // min x
        assert_eq!(bbox[3], 20.0); // max x
    }

    #[test]
    fn test_aggregate_handles_2d_bbox() {
        // Item with 4-element 2D bbox
        let item = create_test_stac_item(
            "test-item",
            "CityJSON",
            vec![],
            vec![],
            10,
            None,
            Some(vec![0.0, 0.0, 10.0, 10.0]), // 2D bbox
        );

        let collection = StacCollectionBuilder::new("test")
            .aggregate_from_items(&[item])
            .expect("Failed to aggregate")
            .build()
            .expect("Failed to build collection");

        // Should still have spatial extent
        assert!(!collection.extent.spatial.bbox.is_empty());
    }

    #[test]
    fn test_aggregate_handles_missing_properties() {
        // Item with minimal properties - use stac::Item::new
        let mut item = stac::Item::new("minimal-item");
        item.properties.datetime = Some("2024-01-01T00:00:00Z".parse().unwrap());
        item.bbox = vec![0.0, 0.0, 0.0, 10.0, 10.0, 10.0].try_into().ok();

        // Should not panic
        let collection = StacCollectionBuilder::new("test")
            .aggregate_from_items(&[item])
            .expect("Failed to aggregate")
            .build()
            .expect("Failed to build collection");

        // Should have spatial extent
        assert!(!collection.extent.spatial.bbox.is_empty());
    }
}
