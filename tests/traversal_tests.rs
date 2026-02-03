//! Unit tests for file traversal

use cityjson_stac::traversal;
use std::fs;
use std::path::Path;
use tempfile::tempdir;

mod traversal_tests {
    use super::*;

    #[test]
    fn test_find_cityjson_files_in_test_data() {
        let test_data = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("data");
        let files = traversal::find_cityjson_files(&test_data).expect("Failed to find files");

        // Should find at least the .json and .jsonl files
        assert!(!files.is_empty());

        // Should find delft.city.json
        let delft = files
            .iter()
            .any(|p| p.file_name().unwrap() == "delft.city.json");
        assert!(delft, "Should find delft.city.json");

        // Should find railway.city.json
        let railway = files
            .iter()
            .any(|p| p.file_name().unwrap() == "railway.city.json");
        assert!(railway, "Should find railway.city.json");

        // Should find jsonl files
        let jsonl_count = files
            .iter()
            .filter(|p| p.extension().map(|e| e == "jsonl").unwrap_or(false))
            .count();
        assert!(jsonl_count >= 2, "Should find .jsonl files");

        // Should find fcb files
        let fcb_count = files
            .iter()
            .filter(|p| p.extension().map(|e| e == "fcb").unwrap_or(false))
            .count();
        assert!(fcb_count >= 1, "Should find .fcb files");
    }

    #[test]
    fn test_find_single_cityjson_file() {
        let test_file = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("data")
            .join("delft.city.json");

        let files = traversal::find_cityjson_files(&test_file).expect("Failed to find files");

        assert_eq!(files.len(), 1);
        assert_eq!(files[0].file_name().unwrap(), "delft.city.json");
    }

    #[test]
    fn test_find_cityjson_files_empty_dir() {
        let temp = tempdir().expect("Failed to create temp dir");
        let files = traversal::find_cityjson_files(temp.path()).expect("Failed to find files");

        assert!(files.is_empty());
    }

    #[test]
    fn test_find_cityjson_files_nonexistent_path() {
        let path = Path::new("/nonexistent/path/to/dir");
        let result = traversal::find_cityjson_files(path);

        assert!(result.is_err());
    }

    #[test]
    fn test_find_cityjson_files_recursive() {
        let temp = tempdir().expect("Failed to create temp dir");

        // Create nested structure
        let subdir = temp.path().join("subdir");
        fs::create_dir(&subdir).expect("Failed to create subdir");

        // Create test files
        fs::write(temp.path().join("root.city.json"), r#"{"type":"CityJSON"}"#)
            .expect("Failed to write file");
        fs::write(subdir.join("nested.city.json"), r#"{"type":"CityJSON"}"#)
            .expect("Failed to write file");

        let files = traversal::find_cityjson_files(temp.path()).expect("Failed to find files");

        assert_eq!(files.len(), 2);
    }

    #[test]
    fn test_find_cityjson_files_filters_extensions() {
        let temp = tempdir().expect("Failed to create temp dir");

        // Create files with various extensions
        fs::write(
            temp.path().join("building.city.json"),
            r#"{"type":"CityJSON"}"#,
        )
        .expect("Failed to write json");
        fs::write(temp.path().join("building.city.jsonl"), "").expect("Failed to write jsonl");
        fs::write(temp.path().join("building.fcb"), "").expect("Failed to write fcb");
        fs::write(temp.path().join("readme.txt"), "").expect("Failed to write txt");
        fs::write(temp.path().join("data.csv"), "").expect("Failed to write csv");

        let files = traversal::find_cityjson_files(temp.path()).expect("Failed to find files");

        // Should only find .json, .jsonl, .fcb files
        assert_eq!(files.len(), 3);

        // Should not find txt or csv
        assert!(!files
            .iter()
            .any(|p| p.extension().map(|e| e == "txt").unwrap_or(false)));
        assert!(!files
            .iter()
            .any(|p| p.extension().map(|e| e == "csv").unwrap_or(false)));
    }
}

mod output_path_tests {
    use super::*;

    #[test]
    fn test_generate_item_output_path() {
        let input = Path::new("/data/buildings/delft.city.json");
        let output_dir = Path::new("/output/stac");

        let result = traversal::generate_output_path(input, output_dir, "item");

        assert!(result.to_string_lossy().contains("delft"));
        assert!(result.extension().map(|e| e == "json").unwrap_or(false));
    }

    #[test]
    fn test_generate_collection_output_path() {
        let output_dir = Path::new("/output/stac");

        let result = traversal::generate_collection_path(output_dir);

        assert_eq!(result, Path::new("/output/stac/collection.json"));
    }

    #[test]
    fn test_output_path_preserves_structure() {
        let input = Path::new("/data/city/area1/buildings.city.json");
        let output_dir = Path::new("/output");
        let basename = Path::new("/data/city");

        let result = traversal::generate_relative_output_path(input, output_dir, basename);

        // Should preserve area1 subdirectory
        assert!(result.to_string_lossy().contains("area1"));
    }
}
