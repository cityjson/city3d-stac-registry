//! Unit tests for metadata structures

use cityjson_stac::metadata::{AttributeDefinition, AttributeType, BBox3D, Transform, CRS};

mod bbox_tests {
    use super::*;

    #[test]
    fn test_bbox_new() {
        let bbox = BBox3D::new(0.0, 1.0, 2.0, 10.0, 11.0, 12.0);
        assert_eq!(bbox.xmin, 0.0);
        assert_eq!(bbox.ymin, 1.0);
        assert_eq!(bbox.zmin, 2.0);
        assert_eq!(bbox.xmax, 10.0);
        assert_eq!(bbox.ymax, 11.0);
        assert_eq!(bbox.zmax, 12.0);
    }

    #[test]
    fn test_bbox_to_array() {
        let bbox = BBox3D::new(1.0, 2.0, 3.0, 4.0, 5.0, 6.0);
        assert_eq!(bbox.to_array(), [1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
    }

    #[test]
    fn test_bbox_footprint_2d() {
        let bbox = BBox3D::new(0.0, 10.0, 100.0, 5.0, 15.0, 200.0);
        assert_eq!(bbox.footprint_2d(), [0.0, 10.0, 5.0, 15.0]);
    }

    #[test]
    fn test_bbox_is_valid() {
        let valid = BBox3D::new(0.0, 0.0, 0.0, 10.0, 10.0, 10.0);
        assert!(valid.is_valid());

        let invalid_x = BBox3D::new(10.0, 0.0, 0.0, 0.0, 10.0, 10.0);
        assert!(!invalid_x.is_valid());

        let invalid_y = BBox3D::new(0.0, 10.0, 0.0, 10.0, 0.0, 10.0);
        assert!(!invalid_y.is_valid());

        let invalid_z = BBox3D::new(0.0, 0.0, 10.0, 10.0, 10.0, 0.0);
        assert!(!invalid_z.is_valid());
    }

    #[test]
    fn test_bbox_center() {
        let bbox = BBox3D::new(0.0, 0.0, 0.0, 10.0, 20.0, 30.0);
        assert_eq!(bbox.center(), (5.0, 10.0, 15.0));
    }

    #[test]
    fn test_bbox_merge() {
        let bbox1 = BBox3D::new(0.0, 0.0, 0.0, 5.0, 5.0, 5.0);
        let bbox2 = BBox3D::new(3.0, 3.0, 3.0, 10.0, 10.0, 10.0);
        let merged = bbox1.merge(&bbox2);

        assert_eq!(merged.xmin, 0.0);
        assert_eq!(merged.ymin, 0.0);
        assert_eq!(merged.zmin, 0.0);
        assert_eq!(merged.xmax, 10.0);
        assert_eq!(merged.ymax, 10.0);
        assert_eq!(merged.zmax, 10.0);
    }

    #[test]
    fn test_bbox_merge_disjoint() {
        let bbox1 = BBox3D::new(0.0, 0.0, 0.0, 1.0, 1.0, 1.0);
        let bbox2 = BBox3D::new(100.0, 100.0, 100.0, 200.0, 200.0, 200.0);
        let merged = bbox1.merge(&bbox2);

        assert_eq!(merged.xmin, 0.0);
        assert_eq!(merged.xmax, 200.0);
    }

    #[test]
    fn test_bbox_serialization() {
        let bbox = BBox3D::new(1.0, 2.0, 3.0, 4.0, 5.0, 6.0);
        let json = serde_json::to_string(&bbox).expect("Failed to serialize");
        let deserialized: BBox3D = serde_json::from_str(&json).expect("Failed to deserialize");
        assert_eq!(bbox, deserialized);
    }

    #[test]
    fn test_bbox_clone() {
        let bbox = BBox3D::new(1.0, 2.0, 3.0, 4.0, 5.0, 6.0);
        let cloned = bbox.clone();
        assert_eq!(bbox, cloned);
    }
}

mod crs_tests {
    use super::*;

    #[test]
    fn test_crs_from_epsg() {
        let crs = CRS::from_epsg(7415);
        assert_eq!(crs.epsg, Some(7415));
        assert_eq!(crs.authority, Some("EPSG".to_string()));
        assert_eq!(crs.identifier, Some("7415".to_string()));
    }

    #[test]
    fn test_crs_from_cityjson_url() {
        let url = "https://www.opengis.net/def/crs/EPSG/0/7415";
        let crs = CRS::from_cityjson_url(url).expect("Failed to parse CRS URL");
        assert_eq!(crs.epsg, Some(7415));
    }

    #[test]
    fn test_crs_from_cityjson_url_wgs84() {
        let url = "https://www.opengis.net/def/crs/EPSG/0/4326";
        let crs = CRS::from_cityjson_url(url).expect("Failed to parse CRS URL");
        assert_eq!(crs.epsg, Some(4326));
    }

    #[test]
    fn test_crs_to_stac_epsg() {
        let crs = CRS::from_epsg(7415);
        assert_eq!(crs.to_stac_epsg(), Some(7415));
    }

    #[test]
    fn test_crs_to_cityjson_url() {
        let crs = CRS::from_epsg(7415);
        let url = crs.to_cityjson_url().expect("Failed to generate URL");
        assert_eq!(url, "https://www.opengis.net/def/crs/EPSG/0/7415");
    }

    #[test]
    fn test_crs_default() {
        let crs = CRS::default();
        assert_eq!(crs.epsg, Some(4326)); // WGS84
    }

    #[test]
    fn test_crs_from_invalid_url() {
        let url = "invalid-url";
        let result = CRS::from_cityjson_url(url);
        assert!(result.is_none());
    }

    #[test]
    fn test_crs_roundtrip() {
        let original = CRS::from_epsg(28992);
        let url = original.to_cityjson_url().expect("Failed to generate URL");
        let parsed = CRS::from_cityjson_url(&url).expect("Failed to parse URL");
        assert_eq!(original.epsg, parsed.epsg);
    }
}

mod transform_tests {
    use super::*;

    #[test]
    fn test_transform_new() {
        let transform = Transform::new([0.001, 0.001, 0.001], [1000.0, 2000.0, 0.0]);
        assert_eq!(transform.scale, [0.001, 0.001, 0.001]);
        assert_eq!(transform.translate, [1000.0, 2000.0, 0.0]);
    }

    #[test]
    fn test_transform_apply() {
        let transform = Transform::new([0.001, 0.001, 0.001], [1000.0, 2000.0, 0.0]);
        let compressed = [100, 200, 300];
        let real = transform.apply(&compressed);

        assert!((real[0] - 1000.1).abs() < 1e-9);
        assert!((real[1] - 2000.2).abs() < 1e-9);
        assert!((real[2] - 0.3).abs() < 1e-9);
    }

    #[test]
    fn test_transform_apply_batch() {
        let transform = Transform::new([0.001, 0.001, 0.001], [0.0, 0.0, 0.0]);
        let compressed = vec![[1000, 2000, 3000], [4000, 5000, 6000]];
        let real = transform.apply_batch(&compressed);

        assert_eq!(real.len(), 2);
        assert!((real[0][0] - 1.0).abs() < 1e-9);
        assert!((real[1][0] - 4.0).abs() < 1e-9);
    }

    #[test]
    fn test_transform_inverse() {
        let transform = Transform::new([0.001, 0.001, 0.001], [1000.0, 2000.0, 0.0]);
        let real = [1000.1, 2000.2, 0.3];
        let compressed = transform.inverse(&real);

        assert_eq!(compressed, [100, 200, 300]);
    }

    #[test]
    fn test_transform_roundtrip() {
        let transform = Transform::new([0.001, 0.001, 0.001], [1000.0, 2000.0, 0.0]);
        let original = [100, 200, 300];
        let real = transform.apply(&original);
        let back = transform.inverse(&real);
        assert_eq!(original, back);
    }

    #[test]
    fn test_transform_serialization() {
        let transform = Transform::new([0.001, 0.002, 0.003], [100.0, 200.0, 300.0]);
        let json = serde_json::to_string(&transform).expect("Failed to serialize");
        let deserialized: Transform = serde_json::from_str(&json).expect("Failed to deserialize");
        assert_eq!(transform, deserialized);
    }

    #[test]
    fn test_transform_with_negative_values() {
        let transform = Transform::new([0.001, 0.001, 0.001], [-1000.0, -2000.0, -100.0]);
        let compressed = [1000, 2000, 300];
        let real = transform.apply(&compressed);

        assert!((real[0] - (-999.0)).abs() < 1e-9);
        assert!((real[1] - (-1998.0)).abs() < 1e-9);
        assert!((real[2] - (-99.7)).abs() < 1e-9);
    }
}

mod attribute_tests {
    use super::*;

    #[test]
    fn test_attribute_definition_new() {
        let attr = AttributeDefinition::new("height", AttributeType::Number);
        assert_eq!(attr.name, "height");
        assert_eq!(attr.attr_type, AttributeType::Number);
        assert!(attr.description.is_none());
        assert!(attr.required.is_none());
    }

    #[test]
    fn test_attribute_definition_with_description() {
        let attr = AttributeDefinition::new("height", AttributeType::Number)
            .with_description("Building height in meters");
        assert_eq!(
            attr.description,
            Some("Building height in meters".to_string())
        );
    }

    #[test]
    fn test_attribute_definition_with_required() {
        let attr = AttributeDefinition::new("id", AttributeType::String).with_required(true);
        assert_eq!(attr.required, Some(true));
    }

    #[test]
    fn test_attribute_definition_builder_chain() {
        let attr = AttributeDefinition::new("function", AttributeType::String)
            .with_description("Building function")
            .with_required(false);

        assert_eq!(attr.name, "function");
        assert_eq!(attr.attr_type, AttributeType::String);
        assert!(attr.description.is_some());
        assert_eq!(attr.required, Some(false));
    }

    #[test]
    fn test_attribute_type_from_json_string() {
        let value = serde_json::json!("hello");
        assert_eq!(
            AttributeType::from_json_value(&value),
            AttributeType::String
        );
    }

    #[test]
    fn test_attribute_type_from_json_number() {
        let value = serde_json::json!(42);
        assert_eq!(
            AttributeType::from_json_value(&value),
            AttributeType::Number
        );

        let float_value = serde_json::json!(3.15);
        assert_eq!(
            AttributeType::from_json_value(&float_value),
            AttributeType::Number
        );
    }

    #[test]
    fn test_attribute_type_from_json_boolean() {
        let value = serde_json::json!(true);
        assert_eq!(
            AttributeType::from_json_value(&value),
            AttributeType::Boolean
        );
    }

    #[test]
    fn test_attribute_type_from_json_array() {
        let value = serde_json::json!([1, 2, 3]);
        assert_eq!(AttributeType::from_json_value(&value), AttributeType::Array);
    }

    #[test]
    fn test_attribute_type_from_json_object() {
        let value = serde_json::json!({"key": "value"});
        assert_eq!(
            AttributeType::from_json_value(&value),
            AttributeType::Object
        );
    }

    #[test]
    fn test_attribute_type_from_json_null() {
        let value = serde_json::json!(null);
        // Null defaults to String
        assert_eq!(
            AttributeType::from_json_value(&value),
            AttributeType::String
        );
    }

    #[test]
    fn test_attribute_serialization() {
        let attr = AttributeDefinition::new("year", AttributeType::Number);
        let json = serde_json::to_string(&attr).expect("Failed to serialize");
        assert!(json.contains("\"name\":\"year\""));
        assert!(json.contains("\"type\":\"Number\""));
    }
}
