//! Unit tests for file traversal

use cityjson_stac::traversal;
use std::fs;
use std::path::Path;
use tempfile::tempdir;

mod find_files_tests {
    use super::*;

    #[test]
    fn test_find_files_in_test_data() {
        let test_data = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("data");
        let files = traversal::find_files(&test_data, true, None).expect("Failed to find files");

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
    fn test_find_files_non_recursive() {
        let temp = tempdir().expect("Failed to create temp dir");

        // Create nested structure
        let subdir = temp.path().join("subdir");
        fs::create_dir(&subdir).expect("Failed to create subdir");

        // Create test files
        fs::write(temp.path().join("root.city.json"), r#"{"type":"CityJSON"}"#)
            .expect("Failed to write file");
        fs::write(subdir.join("nested.city.json"), r#"{"type":"CityJSON"}"#)
            .expect("Failed to write file");

        // Non-recursive should only find root file
        let files = traversal::find_files(temp.path(), false, None).expect("Failed to find files");
        assert_eq!(files.len(), 1);
    }

    #[test]
    fn test_find_files_empty_dir() {
        let temp = tempdir().expect("Failed to create temp dir");
        let files = traversal::find_files(temp.path(), true, None).expect("Failed to find files");

        assert!(files.is_empty());
    }

    #[test]
    fn test_find_files_nonexistent_path() {
        let path = Path::new("/nonexistent/path/to/dir");
        let result = traversal::find_files(path, true, None);

        assert!(result.is_err());
    }

    #[test]
    fn test_find_files_recursive() {
        let temp = tempdir().expect("Failed to create temp dir");

        // Create nested structure
        let subdir = temp.path().join("subdir");
        fs::create_dir(&subdir).expect("Failed to create subdir");

        // Create test files
        fs::write(temp.path().join("root.city.json"), r#"{"type":"CityJSON"}"#)
            .expect("Failed to write file");
        fs::write(subdir.join("nested.city.json"), r#"{"type":"CityJSON"}"#)
            .expect("Failed to write file");

        // Recursive should find both
        let files = traversal::find_files(temp.path(), true, None).expect("Failed to find files");
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn test_find_files_filters_extensions() {
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

        let files = traversal::find_files(temp.path(), true, None).expect("Failed to find files");

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

    #[test]
    fn test_find_files_max_depth() {
        let temp = tempdir().expect("Failed to create temp dir");

        // Create nested directories
        let sub1 = temp.path().join("sub1");
        let sub2 = sub1.join("sub2");
        fs::create_dir_all(&sub2).expect("Failed to create dirs");

        // Create test files at each level
        fs::write(temp.path().join("file0.json"), r#"{}"#).expect("Failed to write file");
        fs::write(sub1.join("file1.json"), r#"{}"#).expect("Failed to write file");
        fs::write(sub2.join("file2.json"), r#"{}"#).expect("Failed to write file");

        // Max depth 2 should find root and sub1, but not sub2
        let files =
            traversal::find_files(temp.path(), true, Some(2)).expect("Failed to find files");
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn test_find_files_sorted() {
        let temp = tempdir().expect("Failed to create temp dir");

        // Create files in non-alphabetical order
        fs::write(temp.path().join("c_file.json"), r#"{}"#).expect("Failed to write file");
        fs::write(temp.path().join("a_file.json"), r#"{}"#).expect("Failed to write file");
        fs::write(temp.path().join("b_file.json"), r#"{}"#).expect("Failed to write file");

        let files = traversal::find_files(temp.path(), true, None).expect("Failed to find files");

        // Should be sorted
        assert_eq!(files[0].file_name().unwrap(), "a_file.json");
        assert_eq!(files[1].file_name().unwrap(), "b_file.json");
        assert_eq!(files[2].file_name().unwrap(), "c_file.json");
    }
}

mod filter_by_extensions_tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_filter_single_extension() {
        let files = vec![
            PathBuf::from("file1.json"),
            PathBuf::from("file2.jsonl"),
            PathBuf::from("file3.fcb"),
        ];

        let filtered = traversal::filter_by_extensions(files, &["json".to_string()]);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].extension().unwrap(), "json");
    }

    #[test]
    fn test_filter_multiple_extensions() {
        let files = vec![
            PathBuf::from("file1.json"),
            PathBuf::from("file2.jsonl"),
            PathBuf::from("file3.fcb"),
        ];

        let filtered =
            traversal::filter_by_extensions(files, &["json".to_string(), "jsonl".to_string()]);
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_filter_case_insensitive() {
        let files = vec![
            PathBuf::from("file1.JSON"),
            PathBuf::from("file2.Json"),
            PathBuf::from("file3.json"),
        ];

        let filtered = traversal::filter_by_extensions(files, &["json".to_string()]);
        assert_eq!(filtered.len(), 3);
    }

    #[test]
    fn test_filter_empty_list() {
        let files: Vec<PathBuf> = vec![];
        let filtered = traversal::filter_by_extensions(files, &["json".to_string()]);
        assert!(filtered.is_empty());
    }

    #[test]
    fn test_filter_no_matches() {
        let files = vec![PathBuf::from("file1.txt"), PathBuf::from("file2.csv")];

        let filtered = traversal::filter_by_extensions(files, &["json".to_string()]);
        assert!(filtered.is_empty());
    }
}
