//! Unit tests for the reader module

use city3d_stac::reader::{get_reader, CityJSONReader, CityJSONSeqReader, CityModelMetadataReader};
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

        // Check the geographicalExtent from delft.city.json metadata
        assert!((bbox.xmin - 84927.558).abs() < 0.001);
        assert!((bbox.ymin - 446572.456).abs() < 0.001);
        assert!((bbox.zmin - (-3.704)).abs() < 0.001);
        assert!((bbox.xmax - 85527.591).abs() < 0.001);
        assert!((bbox.ymax - 447122.446).abs() < 0.001);
        assert!((bbox.zmax - 52.147).abs() < 0.001);
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
        // delft.city.json has 319 CityObjects
        assert_eq!(count, 319);
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
        // delft.city.json has LODs: 0, 1.2, 1.3, 2.2
        assert!(!lods.is_empty());
        assert!(lods.contains(&"0".to_string()));
        assert!(lods.contains(&"1.2".to_string()));
        assert!(lods.contains(&"1.3".to_string()));
        assert!(lods.contains(&"2.2".to_string()));
    }

    #[test]
    fn test_delft_city_object_types_empty() {
        let path = test_data_path("delft.city.json");
        let reader = CityJSONReader::new(&path).expect("Failed to create reader");

        let types = reader.city_object_types().expect("Failed to get types");
        // delft.city.json has Building and BuildingPart types
        assert!(!types.is_empty());
        assert!(types.contains(&"Building".to_string()));
        assert!(types.contains(&"BuildingPart".to_string()));
    }

    #[test]
    fn test_delft_attributes_empty() {
        let path = test_data_path("delft.city.json");
        let reader = CityJSONReader::new(&path).expect("Failed to create reader");

        let attrs = reader.attributes().expect("Failed to get attributes");
        // delft.city.json has many attributes
        assert!(!attrs.is_empty());
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
    fn test_get_reader_fcb_supported() {
        // FlatCityBuf is now supported
        let path = test_data_path("all.fcb");
        let result = get_reader(&path);

        assert!(result.is_ok(), "FlatCityBuf should be supported now");
        let reader = result.unwrap();
        assert_eq!(reader.encoding(), "FlatCityBuf");
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

mod cjseq_integration_tests {
    use super::*;

    #[test]
    fn test_read_delft_cjseq() {
        let path = test_data_path("delft.city.jsonl");
        let reader = get_reader(&path).expect("Failed to create reader");

        assert_eq!(reader.encoding(), "CityJSONSeq");

        let version = reader.version().expect("Failed to get version");
        assert_eq!(version, "2.0");

        let bbox = reader.bbox().expect("Failed to get bbox");
        assert!(bbox.is_valid());
    }

    #[test]
    fn test_read_railway_cjseq() {
        let path = test_data_path("railway.city.jsonl");
        let reader = get_reader(&path).expect("Failed to create reader");

        assert_eq!(reader.encoding(), "CityJSONSeq");

        let count = reader.city_object_count().expect("Failed to get count");
        assert!(count > 0, "Railway should have city objects");

        let types = reader.city_object_types().expect("Failed to get types");
        assert!(!types.is_empty());
    }

    #[test]
    fn test_cjseq_reader_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<CityJSONSeqReader>();
    }

    #[test]
    fn test_delft_cjseq_crs() {
        let path = test_data_path("delft.city.jsonl");
        let reader = get_reader(&path).expect("Failed to create reader");

        let crs = reader.crs().expect("Failed to get CRS");
        assert_eq!(crs.epsg, Some(7415));
    }

    #[test]
    fn test_delft_cjseq_transform() {
        let path = test_data_path("delft.city.jsonl");
        let reader = get_reader(&path).expect("Failed to create reader");

        let transform = reader.transform().expect("Failed to get transform");
        assert!(transform.is_some());
    }
}

mod fcb_integration_tests {
    use super::*;
    use city3d_stac::reader::FlatCityBufReader;

    #[test]
    fn test_read_fcb_file() {
        let path = test_data_path("all.fcb");
        let reader = FlatCityBufReader::new(&path).expect("Failed to create reader");

        assert_eq!(reader.encoding(), "FlatCityBuf");
    }

    #[test]
    fn test_fcb_version() {
        let path = test_data_path("all.fcb");
        let reader = FlatCityBufReader::new(&path).expect("Failed to create reader");

        let version = reader.version().expect("Failed to get version");
        // FCB files have a CityJSON version
        assert!(!version.is_empty(), "Version should not be empty");
    }

    #[test]
    fn test_fcb_city_object_count() {
        let path = test_data_path("all.fcb");
        let reader = FlatCityBufReader::new(&path).expect("Failed to create reader");

        let count = reader.city_object_count().expect("Failed to get count");
        // FCB files have features
        assert!(count > 0, "FCB file should have city objects");
    }

    #[test]
    fn test_fcb_bbox() {
        let path = test_data_path("all.fcb");
        let reader = FlatCityBufReader::new(&path).expect("Failed to create reader");

        let result = reader.bbox();
        // FCB files may or may not have bbox
        if let Ok(bbox) = result {
            assert!(bbox.is_valid(), "Bbox should be valid if present");
        }
    }

    #[test]
    fn test_fcb_crs() {
        let path = test_data_path("all.fcb");
        let reader = FlatCityBufReader::new(&path).expect("Failed to create reader");

        let crs = reader.crs().expect("Failed to get CRS");
        // CRS should always be available (may be default WGS84)
        assert!(crs.epsg.is_some(), "CRS should have EPSG code");
    }

    #[test]
    fn test_fcb_attributes() {
        let path = test_data_path("all.fcb");
        let reader = FlatCityBufReader::new(&path).expect("Failed to create reader");

        let attrs = reader.attributes().expect("Failed to get attributes");
        // FCB files define columns which map to attributes
        // Verify attributes are sorted by name (the reader should sort them)
        if !attrs.is_empty() {
            let names: Vec<_> = attrs.iter().map(|a| &a.name).collect();
            let mut sorted_names = names.clone();
            sorted_names.sort();
            assert_eq!(names, sorted_names, "Attributes should be sorted by name");
        }
    }

    #[test]
    fn test_fcb_transform() {
        let path = test_data_path("all.fcb");
        let reader = FlatCityBufReader::new(&path).expect("Failed to create reader");

        let transform = reader.transform().expect("Failed to get transform");
        // Transform may or may not be present
        if let Some(t) = transform {
            assert_eq!(t.scale.len(), 3, "Scale should have 3 components");
            assert_eq!(t.translate.len(), 3, "Translate should have 3 components");
        }
    }

    #[test]
    fn test_fcb_metadata() {
        let path = test_data_path("all.fcb");
        let reader = FlatCityBufReader::new(&path).expect("Failed to create reader");

        let metadata = reader.metadata().expect("Failed to get metadata");
        // Metadata may or may not be present
        if let Some(m) = metadata {
            assert!(m.is_object(), "Metadata should be an object");
        }
    }

    #[test]
    fn test_fcb_file_path() {
        let path = test_data_path("all.fcb");
        let reader = FlatCityBufReader::new(&path).expect("Failed to create reader");

        assert_eq!(reader.file_path(), path);
    }

    #[test]
    fn test_fcb_thread_safety() {
        use std::sync::Arc;
        use std::thread;

        let path = test_data_path("all.fcb");
        let reader = Arc::new(FlatCityBufReader::new(&path).expect("Failed to create reader"));

        let handles: Vec<_> = (0..4)
            .map(|_| {
                let r = Arc::clone(&reader);
                thread::spawn(move || {
                    let encoding = r.encoding();
                    let version = r.version().expect("Failed to get version");
                    (encoding, version)
                })
            })
            .collect();

        for handle in handles {
            let (encoding, version) = handle.join().expect("Thread panicked");
            assert_eq!(encoding, "FlatCityBuf");
            assert!(!version.is_empty());
        }
    }

    #[test]
    fn test_fcb_lods() {
        let path = test_data_path("all.fcb");
        let reader = FlatCityBufReader::new(&path).expect("Failed to create reader");

        let lods = reader.lods().expect("Failed to get LODs");
        // The all.fcb file contains geometry with LODs
        assert!(
            !lods.is_empty(),
            "FCB file should have at least one LOD from geometry"
        );

        // Verify LODs are sorted alphabetically
        let mut sorted = lods.clone();
        sorted.sort();
        assert_eq!(lods, sorted, "LODs should be sorted");
    }

    #[test]
    fn test_fcb_city_object_types() {
        let path = test_data_path("all.fcb");
        let reader = FlatCityBufReader::new(&path).expect("Failed to create reader");

        let types = reader
            .city_object_types()
            .expect("Failed to get city object types");
        // FCB file should have at least one city object type
        assert!(!types.is_empty(), "FCB file should have city object types");

        // The test file is called "all.fcb" so it likely contains various city object types
        // Types should be sorted alphabetically
        let mut sorted = types.clone();
        sorted.sort();
        assert_eq!(types, sorted, "City object types should be sorted");
    }

    #[test]
    fn test_fcb_lods_and_types_caching() {
        let path = test_data_path("all.fcb");
        let reader = FlatCityBufReader::new(&path).expect("Failed to create reader");

        // Call lods() twice - second call should use cache
        let lods1 = reader.lods().expect("Failed to get LODs first time");
        let lods2 = reader.lods().expect("Failed to get LODs second time");
        assert_eq!(lods1, lods2, "LODs should be consistent between calls");

        // city_object_types() should use the same cached data
        let types = reader
            .city_object_types()
            .expect("Failed to get city object types");
        assert!(!types.is_empty(), "Should get types from cached data");
    }

    #[test]
    fn test_fcb_streaming_thread_safety() {
        use std::sync::Arc;
        use std::thread;

        let path = test_data_path("all.fcb");
        let reader = Arc::new(FlatCityBufReader::new(&path).expect("Failed to create reader"));

        let handles: Vec<_> = (0..4)
            .map(|i| {
                let r = Arc::clone(&reader);
                thread::spawn(move || {
                    if i % 2 == 0 {
                        r.lods().expect("Failed to get LODs")
                    } else {
                        r.city_object_types().expect("Failed to get types")
                    }
                })
            })
            .collect();

        // Get baseline results for comparison
        let baseline_lods = reader.lods().expect("Failed to get baseline LODs");
        let baseline_types = reader
            .city_object_types()
            .expect("Failed to get baseline types");

        for (i, handle) in handles.into_iter().enumerate() {
            let result = handle.join().expect("Thread panicked");
            // Verify each thread got the same result as sequential baseline
            if i % 2 == 0 {
                assert_eq!(result, baseline_lods, "LODs should match baseline");
            } else {
                assert_eq!(result, baseline_types, "Types should match baseline");
            }
        }
    }
}
