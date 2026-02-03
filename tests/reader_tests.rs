//! Unit tests for the reader module

use cityjson_stac::reader::{get_reader, CityJSONReader, CityModelMetadataReader};
use std::path::Path;

/// Test data directory path
fn test_data_path(filename: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("data")
        .join(filename)
}

mod cityjson_reader_tests {
    use super::*;

    #[test]
    fn test_read_delft_cityjson() {
        let path = test_data_path("delft.city.json");
        let reader = CityJSONReader::new(&path).expect("Failed to create reader");

        assert_eq!(reader.encoding(), "CityJSON");
    }

    #[test]
    fn test_delft_version() {
        let path = test_data_path("delft.city.json");
        let reader = CityJSONReader::new(&path).expect("Failed to create reader");

        let version = reader.version().expect("Failed to get version");
        assert_eq!(version, "2.0");
    }

    #[test]
    fn test_delft_bbox() {
        let path = test_data_path("delft.city.json");
        let reader = CityJSONReader::new(&path).expect("Failed to create reader");

        let bbox = reader.bbox().expect("Failed to get bbox");

        // Check the geographicalExtent from delft.city.json
        assert!((bbox.xmin - 74782.684).abs() < 0.001);
        assert!((bbox.ymin - 419982.871).abs() < 0.001);
        assert!((bbox.zmin - (-14.93)).abs() < 0.001);
        assert!((bbox.xmax - 100067.947).abs() < 0.001);
        assert!((bbox.ymax - 450171.531).abs() < 0.001);
        assert!((bbox.zmax - 207.042).abs() < 0.001);
    }

    #[test]
    fn test_delft_crs() {
        let path = test_data_path("delft.city.json");
        let reader = CityJSONReader::new(&path).expect("Failed to create reader");

        let crs = reader.crs().expect("Failed to get CRS");
        assert_eq!(crs.epsg, Some(7415));
    }

    #[test]
    fn test_delft_city_object_count() {
        let path = test_data_path("delft.city.json");
        let reader = CityJSONReader::new(&path).expect("Failed to create reader");

        let count = reader
            .city_object_count()
            .expect("Failed to get city object count");
        // delft.city.json has empty CityObjects
        assert_eq!(count, 0);
    }

    #[test]
    fn test_delft_transform() {
        let path = test_data_path("delft.city.json");
        let reader = CityJSONReader::new(&path).expect("Failed to create reader");

        let transform = reader.transform().expect("Failed to get transform");
        assert!(transform.is_some());

        let t = transform.unwrap();
        assert_eq!(t.scale, [0.001, 0.001, 0.001]);
    }

    #[test]
    fn test_delft_metadata() {
        let path = test_data_path("delft.city.json");
        let reader = CityJSONReader::new(&path).expect("Failed to create reader");

        let metadata = reader.metadata().expect("Failed to get metadata");
        assert!(metadata.is_some());

        let m = metadata.unwrap();
        assert_eq!(m["title"], "3DBAG");
    }

    #[test]
    fn test_delft_lods_empty() {
        let path = test_data_path("delft.city.json");
        let reader = CityJSONReader::new(&path).expect("Failed to create reader");

        let lods = reader.lods().expect("Failed to get LODs");
        // delft.city.json has no geometry, so no LODs
        assert!(lods.is_empty());
    }

    #[test]
    fn test_delft_city_object_types_empty() {
        let path = test_data_path("delft.city.json");
        let reader = CityJSONReader::new(&path).expect("Failed to create reader");

        let types = reader.city_object_types().expect("Failed to get types");
        // delft.city.json has no CityObjects
        assert!(types.is_empty());
    }

    #[test]
    fn test_delft_attributes_empty() {
        let path = test_data_path("delft.city.json");
        let reader = CityJSONReader::new(&path).expect("Failed to create reader");

        let attrs = reader.attributes().expect("Failed to get attributes");
        // delft.city.json has no attributes
        assert!(attrs.is_empty());
    }

    #[test]
    fn test_file_path() {
        let path = test_data_path("delft.city.json");
        let reader = CityJSONReader::new(&path).expect("Failed to create reader");

        assert_eq!(reader.file_path(), path);
    }
}

mod railway_tests {
    use super::*;

    #[test]
    fn test_read_railway_cityjson() {
        let path = test_data_path("railway.city.json");
        let reader = CityJSONReader::new(&path).expect("Failed to create reader");

        assert_eq!(reader.encoding(), "CityJSON");
    }

    #[test]
    fn test_railway_version() {
        let path = test_data_path("railway.city.json");
        let reader = CityJSONReader::new(&path).expect("Failed to create reader");

        let version = reader.version().expect("Failed to get version");
        assert_eq!(version, "2.0");
    }

    #[test]
    fn test_railway_has_city_objects() {
        let path = test_data_path("railway.city.json");
        let reader = CityJSONReader::new(&path).expect("Failed to create reader");

        let count = reader.city_object_count().expect("Failed to get count");
        assert!(count > 0, "Railway should have city objects");
    }

    #[test]
    fn test_railway_has_city_object_types() {
        let path = test_data_path("railway.city.json");
        let reader = CityJSONReader::new(&path).expect("Failed to create reader");

        let types = reader.city_object_types().expect("Failed to get types");
        assert!(!types.is_empty(), "Railway should have object types");
    }

    #[test]
    fn test_railway_bbox() {
        let path = test_data_path("railway.city.json");
        let reader = CityJSONReader::new(&path).expect("Failed to create reader");

        let bbox = reader.bbox().expect("Failed to get bbox");
        assert!(bbox.is_valid());
    }

    #[test]
    fn test_railway_has_lods() {
        let path = test_data_path("railway.city.json");
        let reader = CityJSONReader::new(&path).expect("Failed to create reader");

        let lods = reader.lods().expect("Failed to get LODs");
        assert!(!lods.is_empty(), "Railway should have LODs");
    }
}

mod get_reader_tests {
    use super::*;

    #[test]
    fn test_get_reader_for_cityjson() {
        let path = test_data_path("delft.city.json");
        let reader = get_reader(&path).expect("Failed to get reader");

        assert_eq!(reader.encoding(), "CityJSON");
    }

    #[test]
    fn test_get_reader_unsupported_extension() {
        let path = Path::new("test.txt");
        let result = get_reader(path);

        assert!(result.is_err());
    }

    #[test]
    fn test_get_reader_nonexistent_file() {
        let path = Path::new("nonexistent.json");
        let result = get_reader(path);

        assert!(result.is_err());
    }

    #[test]
    fn test_get_reader_jsonl_not_yet_supported() {
        // CityJSON Sequences should return "not yet supported" error
        let path = test_data_path("delft.city.jsonl");
        let result = get_reader(&path);

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("coming soon") || err.contains("not yet"));
    }

    #[test]
    fn test_get_reader_fcb_not_yet_supported() {
        // FlatCityBuf should return "not yet supported" error
        let path = test_data_path("all.fcb");
        let result = get_reader(&path);

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("coming soon") || err.contains("not yet"));
    }
}

mod reader_thread_safety_tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn test_reader_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<CityJSONReader>();
    }

    #[test]
    fn test_concurrent_reader_access() {
        let path = test_data_path("delft.city.json");
        let reader = Arc::new(CityJSONReader::new(&path).expect("Failed to create reader"));

        let handles: Vec<_> = (0..4)
            .map(|_| {
                let r = Arc::clone(&reader);
                thread::spawn(move || {
                    let version = r.version().expect("Failed to get version");
                    let bbox = r.bbox().expect("Failed to get bbox");
                    (version, bbox)
                })
            })
            .collect();

        for handle in handles {
            let (version, bbox) = handle.join().expect("Thread panicked");
            assert_eq!(version, "2.0");
            assert!(bbox.is_valid());
        }
    }
}
