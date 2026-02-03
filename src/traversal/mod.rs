//! Directory traversal module

use crate::error::{CityJsonStacError, Result};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Find all supported CityJSON files in a directory
///
/// # Arguments
/// * `directory` - Directory to scan
/// * `recursive` - Whether to scan subdirectories recursively
/// * `max_depth` - Maximum directory depth (None for unlimited)
///
/// # Returns
/// Vector of file paths for supported formats
pub fn find_files(
    directory: &Path,
    recursive: bool,
    max_depth: Option<usize>,
) -> Result<Vec<PathBuf>> {
    if !directory.exists() {
        return Err(CityJsonStacError::IoError(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Directory not found: {}", directory.display()),
        )));
    }

    if !directory.is_dir() {
        return Err(CityJsonStacError::IoError(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("Not a directory: {}", directory.display()),
        )));
    }

    let mut walker = WalkDir::new(directory);

    if !recursive {
        walker = walker.max_depth(1);
    } else if let Some(depth) = max_depth {
        walker = walker.max_depth(depth);
    }

    let mut files = Vec::new();

    for entry in walker.into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_file() {
            let path = entry.path();

            // Check if file has a supported extension
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                match ext.to_lowercase().as_str() {
                    "json" | "jsonl" | "cjseq" | "fcb" | "parquet" => {
                        files.push(path.to_path_buf());
                    }
                    _ => {}
                }
            }
        }
    }

    // Sort files for consistent ordering
    files.sort();

    Ok(files)
}

/// Filter files by specific extensions
pub fn filter_by_extensions(files: Vec<PathBuf>, extensions: &[String]) -> Vec<PathBuf> {
    files
        .into_iter()
        .filter(|path| {
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                extensions.iter().any(|e| e.eq_ignore_ascii_case(ext))
            } else {
                false
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_find_files_basic() {
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path();

        // Create test files
        fs::write(dir_path.join("file1.json"), "{}").unwrap();
        fs::write(dir_path.join("file2.jsonl"), "").unwrap();
        fs::write(dir_path.join("file3.txt"), "").unwrap();
        fs::write(dir_path.join("file4.fcb"), "").unwrap();

        let files = find_files(dir_path, false, None).unwrap();

        // Should find .json, .jsonl, and .fcb, but not .txt
        assert_eq!(files.len(), 3);
    }

    #[test]
    fn test_find_files_recursive() {
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path();

        // Create subdirectory
        let sub_dir = dir_path.join("subdir");
        fs::create_dir(&sub_dir).unwrap();

        // Create files in root and subdirectory
        fs::write(dir_path.join("file1.json"), "{}").unwrap();
        fs::write(sub_dir.join("file2.json"), "{}").unwrap();

        // Non-recursive should find only root files
        let files = find_files(dir_path, false, None).unwrap();
        assert_eq!(files.len(), 1);

        // Recursive should find both
        let files = find_files(dir_path, true, None).unwrap();
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn test_find_files_max_depth() {
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path();

        // Create nested directories
        let sub1 = dir_path.join("sub1");
        let sub2 = sub1.join("sub2");
        fs::create_dir_all(&sub2).unwrap();

        fs::write(dir_path.join("file0.json"), "{}").unwrap();
        fs::write(sub1.join("file1.json"), "{}").unwrap();
        fs::write(sub2.join("file2.json"), "{}").unwrap();

        // Max depth 2 should find root and sub1, but not sub2
        let files = find_files(dir_path, true, Some(2)).unwrap();
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn test_filter_by_extensions() {
        let files = vec![
            PathBuf::from("file1.json"),
            PathBuf::from("file2.jsonl"),
            PathBuf::from("file3.fcb"),
        ];

        let filtered = filter_by_extensions(files, &["json".to_string()]);
        assert_eq!(filtered.len(), 1);

        let files = vec![
            PathBuf::from("file1.json"),
            PathBuf::from("file2.jsonl"),
            PathBuf::from("file3.fcb"),
        ];

        let filtered = filter_by_extensions(files, &["json".to_string(), "jsonl".to_string()]);
        assert_eq!(filtered.len(), 2);
    }
}
