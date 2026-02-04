//! Unit tests for STAC item and collection building

use cityjson_stac::metadata::BBox3D;
use cityjson_stac::reader::{CityJSONReader, CityModelMetadataReader};
use cityjson_stac::stac::{Asset, Link, StacCollectionBuilder, StacItemBuilder};
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
                "cj:encoding".to_string(),
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
                "cj:encoding".to_string(),
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
                "cj:encoding".to_string(),
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
                "cj:encoding".to_string(),
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
                "cj:encoding".to_string(),
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
                "cj:encoding".to_string(),
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
                "cj:encoding".to_string(),
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
    fn test_item_builder_requires_encoding() {
        // Without cj:encoding, build should fail
        let result = StacItemBuilder::new("test-id").build();

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("cj:encoding"));
    }

    #[test]
    fn test_item_builder_stac_extensions() {
        let item = StacItemBuilder::new("test-id")
            .property(
                "cj:encoding".to_string(),
                Value::String("CityJSON".to_string()),
            )
            .build()
            .expect("Failed to build item");

        // Should include CityJSON extension
        assert!(item.stac_extensions.iter().any(|e| e.contains("cityjson")));
        // Should include projection extension
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
        assert_eq!(item.properties.get("cj:encoding").unwrap(), "CityJSON");
        assert_eq!(item.properties.get("cj:version").unwrap(), "2.0");
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
        let city_objects = item.properties.get("cj:city_objects");
        assert!(city_objects.is_some());
        assert!(city_objects.unwrap().as_u64().unwrap() > 0);

        // Railway should have LODs
        let lods = item.properties.get("cj:lods");
        assert!(lods.is_some());

        // Railway should have object types
        let types = item.properties.get("cj:co_types");
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
            .summary("cj:lods", serde_json::json!(["1", "2", "2.2"]))
            .spatial_extent(bbox)
            .build()
            .expect("Failed to build collection");

        assert!(collection.summaries.is_some());
        let summaries = collection.summaries.unwrap();
        assert!(summaries.contains_key("cj:lods"));
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
        let summaries = collection.summaries.unwrap();
        assert!(summaries.contains_key("cj:encoding"));
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

        let summaries = collection.summaries.unwrap();

        // Both use CityJSON encoding
        let encodings = summaries.get("cj:encoding").unwrap().as_array().unwrap();
        assert!(encodings.iter().any(|e| e.as_str().unwrap() == "CityJSON"));

        // Should have merged bbox
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
