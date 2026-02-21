//! Unit tests for STAC item and collection building

use city3d_stac::metadata::BBox3D;
use city3d_stac::reader::{CityJSONReader, CityModelMetadataReader};
use city3d_stac::stac::{Asset, Link, StacCollectionBuilder, StacItemBuilder};
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
            .property(
                "city3d:encoding".to_string(),
                Value::String("CityJSON".to_string()),
            )
            .build()
            .expect("Failed to build item");

        assert_eq!(item.id, "test-id");
        assert_eq!(item.stac_version, "1.0.0");
        assert_eq!(item.item_type, "Feature");
    }

    #[test]
    fn test_item_builder_with_bbox() {
        let bbox = BBox3D::new(0.0, 0.0, 0.0, 10.0, 10.0, 10.0);
        let item = StacItemBuilder::new("test-id")
            .bbox(bbox)
            .property(
                "city3d:encoding".to_string(),
                Value::String("CityJSON".to_string()),
            )
            .build()
            .expect("Failed to build item");

        assert!(item.bbox.is_some());
        let bbox_array = item.bbox.unwrap();
        assert_eq!(bbox_array.len(), 6);
        assert_eq!(bbox_array[0], 0.0);
        assert_eq!(bbox_array[5], 10.0);
    }

    #[test]
    fn test_item_builder_with_geometry_from_bbox() {
        let bbox = BBox3D::new(0.0, 0.0, 0.0, 10.0, 10.0, 10.0);
        let item = StacItemBuilder::new("test-id")
            .bbox(bbox)
            .geometry_from_bbox()
            .property(
                "city3d:encoding".to_string(),
                Value::String("CityJSON".to_string()),
            )
            .build()
            .expect("Failed to build item");

        assert!(item.geometry.is_some());
        let geom = item.geometry.unwrap();
        assert_eq!(geom["type"], "Polygon");
    }

    #[test]
    fn test_item_builder_with_title_and_description() {
        let item = StacItemBuilder::new("test-id")
            .title("Test Building")
            .description("A test building dataset")
            .property(
                "city3d:encoding".to_string(),
                Value::String("CityJSON".to_string()),
            )
            .build()
            .expect("Failed to build item");

        assert_eq!(item.properties.get("title").unwrap(), "Test Building");
        assert_eq!(
            item.properties.get("description").unwrap(),
            "A test building dataset"
        );
    }

    #[test]
    fn test_item_builder_has_datetime() {
        let item = StacItemBuilder::new("test-id")
            .property(
                "city3d:encoding".to_string(),
                Value::String("CityJSON".to_string()),
            )
            .build()
            .expect("Failed to build item");

        // datetime is set by default
        assert!(item.properties.contains_key("datetime"));
    }

    #[test]
    fn test_item_builder_with_data_asset() {
        let item = StacItemBuilder::new("test-id")
            .property(
                "city3d:encoding".to_string(),
                Value::String("CityJSON".to_string()),
            )
            .data_asset("./data.json", "application/json")
            .build()
            .expect("Failed to build item");

        assert!(item.assets.contains_key("data"));
        let asset = &item.assets["data"];
        assert_eq!(asset.href, "./data.json");
        assert_eq!(asset.media_type, Some("application/json".to_string()));
    }

    #[test]
    fn test_item_builder_with_links() {
        let item = StacItemBuilder::new("test-id")
            .property(
                "city3d:encoding".to_string(),
                Value::String("CityJSON".to_string()),
            )
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
        // (city3d:encoding is set by cityjson_metadata, not required at the raw builder level)
        let result = StacItemBuilder::new("test-id").build();
        assert!(result.is_ok());
    }

    #[test]
    fn test_item_builder_stac_extensions() {
        // Test with only city3d:encoding - no projection extension
        let item = StacItemBuilder::new("test-id")
            .property(
                "city3d:encoding".to_string(),
                Value::String("CityJSON".to_string()),
            )
            .build()
            .expect("Failed to build item");

        // Should include 3D City Models extension
        assert!(item
            .stac_extensions
            .iter()
            .any(|e| e.contains("stac-city3d")));
        // Should NOT include projection extension (no proj:epsg property)
        assert!(!item
            .stac_extensions
            .iter()
            .any(|e| e.contains("projection")));

        // Test with proj:epsg - projection extension should be included
        let item = StacItemBuilder::new("test-id")
            .property(
                "city3d:encoding".to_string(),
                Value::String("CityJSON".to_string()),
            )
            .property(
                "proj:epsg".to_string(),
                Value::Number(serde_json::Number::from(4326)),
            )
            .build()
            .expect("Failed to build item");

        // Should include both extensions
        assert!(item
            .stac_extensions
            .iter()
            .any(|e| e.contains("stac-city3d")));
        assert!(item
            .stac_extensions
            .iter()
            .any(|e| e.contains("projection")));
    }
}

mod stac_item_from_file_tests {
    use super::*;

    #[test]
    fn test_item_from_delft_file() {
        let path = test_data_path("delft.city.json");
        let reader = CityJSONReader::new(&path).expect("Failed to create reader");

        let builder =
            StacItemBuilder::from_file(&path, &reader, None).expect("Failed to create builder");
        let item = builder.build().expect("Failed to build item");

        // Check CityJSON extension properties

        assert_eq!(item.properties.get("city3d:version").unwrap(), "2.0");
        assert_eq!(item.properties.get("proj:epsg").unwrap(), 7415);

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

        let builder =
            StacItemBuilder::from_file(&path, &reader, None).expect("Failed to create builder");
        let item = builder.build().expect("Failed to build item");

        // Railway should have city objects
        let city_objects = item.properties.get("city3d:city_objects");
        assert!(city_objects.is_some());
        assert!(city_objects.unwrap().as_u64().unwrap() > 0);

        // Railway should have LODs
        let lods = item.properties.get("city3d:lods");
        assert!(lods.is_some());

        // Railway should have object types
        let types = item.properties.get("city3d:co_types");
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
        assert_eq!(collection.stac_version, "1.0.0");
        assert_eq!(collection.collection_type, "Collection");
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
        assert_eq!(
            collection.description,
            Some("A test collection of CityJSON files".to_string())
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
    use super::*;

    #[test]
    fn test_link_new() {
        let link = Link::new("self", "./item.json");
        assert_eq!(link.rel, "self");
        assert_eq!(link.href, "./item.json");
        assert!(link.link_type.is_none());
        assert!(link.title.is_none());
    }

    #[test]
    fn test_link_with_type() {
        let link = Link::new("self", "./item.json").with_type("application/json");
        assert_eq!(link.link_type, Some("application/json".to_string()));
    }

    #[test]
    fn test_link_with_title() {
        let link = Link::new("item", "./item.json").with_title("Building Item");
        assert_eq!(link.title, Some("Building Item".to_string()));
    }

    #[test]
    fn test_link_builder_chain() {
        let link = Link::new("collection", "./collection.json")
            .with_type("application/json")
            .with_title("Parent Collection");

        assert_eq!(link.rel, "collection");
        assert_eq!(link.link_type, Some("application/json".to_string()));
        assert_eq!(link.title, Some("Parent Collection".to_string()));
    }
}

mod asset_tests {
    use super::*;

    #[test]
    fn test_asset_new() {
        let asset = Asset::new("./data.json");
        assert_eq!(asset.href, "./data.json");
        assert!(asset.media_type.is_none());
        assert!(asset.title.is_none());
        assert!(asset.roles.is_none());
    }

    #[test]
    fn test_asset_with_type() {
        let asset = Asset::new("./data.json").with_type("application/json");
        assert_eq!(asset.media_type, Some("application/json".to_string()));
    }

    #[test]
    fn test_asset_with_title() {
        let asset = Asset::new("./data.json").with_title("CityJSON Data");
        assert_eq!(asset.title, Some("CityJSON Data".to_string()));
    }

    #[test]
    fn test_asset_with_roles() {
        let asset = Asset::new("./data.json").with_roles(vec!["data".to_string()]);
        assert_eq!(asset.roles, Some(vec!["data".to_string()]));
    }

    #[test]
    fn test_asset_builder_chain() {
        let asset = Asset::new("./building.json")
            .with_type("application/json")
            .with_title("Building Data")
            .with_roles(vec!["data".to_string(), "primary".to_string()]);

        assert_eq!(asset.href, "./building.json");
        assert_eq!(asset.media_type, Some("application/json".to_string()));
        assert_eq!(asset.title, Some("Building Data".to_string()));
        assert_eq!(
            asset.roles,
            Some(vec!["data".to_string(), "primary".to_string()])
        );
    }
}

mod stac_collection_aggregate_from_items_tests {
    use super::*;
    use city3d_stac::stac::StacItem;
    use std::collections::HashMap;

    /// Helper to create a test STAC item with CityJSON properties
    fn create_test_stac_item(
        id: &str,
        encoding: &str,
        lods: Vec<&str>,
        co_types: Vec<&str>,
        city_objects: i64,
        epsg: Option<i64>,
        bbox: Option<Vec<f64>>,
    ) -> StacItem {
        let mut properties: HashMap<String, Value> = HashMap::new();

        properties.insert(
            "datetime".to_string(),
            Value::String("2024-01-01T00:00:00Z".to_string()),
        );
        properties.insert(
            "city3d:encoding".to_string(),
            Value::String(encoding.to_string()),
        );
        properties.insert(
            "city3d:version".to_string(),
            Value::String("2.0".to_string()),
        );
        properties.insert(
            "city3d:city_objects".to_string(),
            Value::Number(serde_json::Number::from(city_objects)),
        );

        if !lods.is_empty() {
            properties.insert(
                "city3d:lods".to_string(),
                serde_json::to_value(lods).unwrap(),
            );
        }

        if !co_types.is_empty() {
            properties.insert(
                "city3d:co_types".to_string(),
                serde_json::to_value(co_types).unwrap(),
            );
        }

        if let Some(epsg_code) = epsg {
            properties.insert(
                "proj:epsg".to_string(),
                Value::Number(serde_json::Number::from(epsg_code)),
            );
        }

        StacItem {
            stac_version: "1.0.0".to_string(),
            stac_extensions: vec![],
            item_type: "Feature".to_string(),
            id: id.to_string(),
            bbox,
            geometry: None,
            properties,
            assets: HashMap::new(),
            links: vec![],
        }
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

        // Should have both EPSG codes
        let epsg = summaries.get("proj:epsg").unwrap().as_array().unwrap();
        assert_eq!(epsg.len(), 2);

        // Should have merged bbox
        let bbox = &collection.extent.spatial.bbox[0];
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
        // Item with minimal properties
        let mut properties: HashMap<String, Value> = HashMap::new();
        properties.insert(
            "datetime".to_string(),
            Value::String("2024-01-01T00:00:00Z".to_string()),
        );
        properties.insert(
            "city3d:encoding".to_string(),
            Value::String("CityJSON".to_string()),
        );

        let item = StacItem {
            stac_version: "1.0.0".to_string(),
            stac_extensions: vec![],
            item_type: "Feature".to_string(),
            id: "minimal-item".to_string(),
            bbox: Some(vec![0.0, 0.0, 0.0, 10.0, 10.0, 10.0]),
            geometry: None,
            properties,
            assets: HashMap::new(),
            links: vec![],
        };

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
