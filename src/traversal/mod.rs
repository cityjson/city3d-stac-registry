//! Directory traversal module with glob pattern support

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

/// Find files from multiple input sources with pattern matching
///
/// # Arguments
/// * `inputs` - Multiple input paths (files, directories, or glob patterns)
/// * `include_patterns` - Glob patterns to include (e.g., ["*.json", "*.jsonl"])
/// * `exclude_patterns` - Glob patterns to exclude (e.g., ["*test*", "*.bak"])
/// * `recursive` - Whether to scan subdirectories recursively
/// * `max_depth` - Maximum directory depth (None for unlimited)
///
/// # Returns
/// Vector of file paths for supported formats
pub fn find_files_with_patterns(
    inputs: &[PathBuf],
    include_patterns: &[String],
    exclude_patterns: &[String],
    recursive: bool,
    max_depth: Option<usize>,
) -> Result<Vec<PathBuf>> {
    let mut all_files = Vec::new();

    for input in inputs {
        if !input.exists() {
            // Check if it's a glob pattern (contains wildcards)
            let input_str = input.to_string_lossy();
            if input_str.contains('*') || input_str.contains('?') {
                let glob_files =
                    expand_glob_pattern(&input_str, include_patterns, exclude_patterns)?;
                all_files.extend(glob_files);
            }
            continue;
        }

        if input.is_file() {
            // Direct file path - check if it has a supported extension
            if is_supported_file(input) {
                all_files.push(input.clone());
            }
        } else if input.is_dir() {
            // Directory - scan with patterns
            let dir_files = find_in_dir_with_patterns(
                input,
                include_patterns,
                exclude_patterns,
                recursive,
                max_depth,
            )?;
            all_files.extend(dir_files);
        }
    }

    // Remove duplicates while preserving order
    let mut seen = std::collections::HashSet::new();
    all_files.retain(|f| seen.insert(f.clone()));

    // Sort files for consistent ordering
    all_files.sort();

    Ok(all_files)
}

/// Expand a glob pattern and find matching files
fn expand_glob_pattern(
    pattern: &str,
    include_patterns: &[String],
    exclude_patterns: &[String],
) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    // Use glob crate to expand the pattern
    match glob::glob(pattern) {
        Ok(paths) => {
            for entry in paths.filter_map(|e| e.ok()) {
                let path = entry.as_path();
                if path.is_file() && is_supported_file(path) {
                    // Apply include/exclude filters based on filename
                    let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    if matches_patterns(filename, include_patterns, exclude_patterns) {
                        files.push(path.to_path_buf());
                    }
                }
            }
        }
        Err(e) => {
            log::warn!("Invalid glob pattern '{pattern}': {e}");
        }
    }

    Ok(files)
}

/// Find files in a directory with include/exclude patterns
fn find_in_dir_with_patterns(
    directory: &Path,
    include_patterns: &[String],
    exclude_patterns: &[String],
    recursive: bool,
    max_depth: Option<usize>,
) -> Result<Vec<PathBuf>> {
    if !directory.exists() {
        return Err(CityJsonStacError::IoError(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Directory not found: {}", directory.display()),
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
            if !is_supported_file(path) {
                continue;
            }

            // Apply include/exclude filters
            let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

            if matches_patterns(filename, include_patterns, exclude_patterns) {
                files.push(path.to_path_buf());
            }
        }
    }

    Ok(files)
}

/// Check if a filename matches include/exclude patterns
fn matches_patterns(
    filename: &str,
    include_patterns: &[String],
    exclude_patterns: &[String],
) -> bool {
    // Check include patterns (if specified)
    let include_match = if include_patterns.is_empty() {
        true
    } else {
        include_patterns
            .iter()
            .any(|pattern| matches_glob_pattern(filename, pattern))
    };

    if !include_match {
        return false;
    }

    // Check exclude patterns
    !exclude_patterns
        .iter()
        .any(|pattern| matches_glob_pattern(filename, pattern))
}

/// Simple glob pattern matching (supports * and ? wildcards)
fn matches_glob_pattern(text: &str, pattern: &str) -> bool {
    // Convert glob pattern to regex
    let mut regex_str = String::new();

    for c in pattern.chars() {
        match c {
            '*' => regex_str.push_str(".*"),
            '?' => regex_str.push('.'),
            '.' | '+' | '(' | ')' | '[' | ']' | '{' | '}' | '|' | '^' | '$' | '\\' => {
                regex_str.push('\\');
                regex_str.push(c);
            }
            _ => regex_str.push(c),
        }
    }

    // Anchor at start and end
    let anchored = format!("^{regex_str}$");

    if let Ok(re) = regex::Regex::new(&anchored) {
        re.is_match(text)
    } else {
        false
    }
}

/// Check if a file has a supported extension
fn is_supported_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|ext| {
            matches!(
                ext.to_lowercase().as_str(),
                "json" | "jsonl" | "cjseq" | "fcb" | "parquet"
            )
        })
        .unwrap_or(false)
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

    #[test]
    fn test_find_files_with_patterns_include_only() {
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path();

        // Create test files with different extensions
        fs::write(dir_path.join("file1.json"), "{}").unwrap();
        fs::write(dir_path.join("file2.jsonl"), "").unwrap();
        fs::write(dir_path.join("file3.txt"), "").unwrap();
        fs::write(dir_path.join("file4.fcb"), "").unwrap();

        // Include only .json files
        let files = find_files_with_patterns(
            &[dir_path.to_path_buf()],
            &[String::from("*.json")],
            &[],
            false,
            None,
        )
        .unwrap();

        assert_eq!(files.len(), 1);
        assert!(files[0].ends_with("file1.json"));
    }

    #[test]
    fn test_find_files_with_patterns_exclude() {
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path();

        // Create test files
        fs::write(dir_path.join("file1.json"), "{}").unwrap();
        fs::write(dir_path.join("file2_test.json"), "{}").unwrap();
        fs::write(dir_path.join("file3.json"), "{}").unwrap();

        // Exclude files with "_test" in name
        let files = find_files_with_patterns(
            &[dir_path.to_path_buf()],
            &[],
            &[String::from("*_test*")],
            false,
            None,
        )
        .unwrap();

        assert_eq!(files.len(), 2);
        assert!(!files.iter().any(|f| f.to_string_lossy().contains("_test")));
    }

    #[test]
    fn test_find_files_with_patterns_multiple_inputs() {
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path();

        // Create files in root
        fs::write(dir_path.join("file1.json"), "{}").unwrap();
        fs::write(dir_path.join("file2.json"), "{}").unwrap();

        // Create subdirectory with files
        let sub_dir = dir_path.join("subdir");
        fs::create_dir(&sub_dir).unwrap();
        fs::write(sub_dir.join("file3.json"), "{}").unwrap();

        // Use both directory and specific file as inputs
        let inputs = vec![dir_path.to_path_buf(), sub_dir.join("file3.json")];
        let files = find_files_with_patterns(&inputs, &[], &[], true, None).unwrap();

        // Should find all 3 files without duplicates
        assert_eq!(files.len(), 3);
    }

    #[test]
    fn test_is_supported_file() {
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path();

        // Create test files
        fs::write(dir_path.join("file.json"), "{}").unwrap();
        fs::write(dir_path.join("file.jsonl"), "").unwrap();
        fs::write(dir_path.join("file.fcb"), "").unwrap();
        fs::write(dir_path.join("file.txt"), "").unwrap();
        fs::write(dir_path.join("file.parquet"), "").unwrap();

        // Check supported files
        assert!(is_supported_file(&dir_path.join("file.json")));
        assert!(is_supported_file(&dir_path.join("file.jsonl")));
        assert!(is_supported_file(&dir_path.join("file.fcb")));
        assert!(is_supported_file(&dir_path.join("file.parquet")));
        assert!(!is_supported_file(&dir_path.join("file.txt")));
    }
}
