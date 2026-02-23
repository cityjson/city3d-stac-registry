//! Validation result types

/// Result of dry-run validation
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Config file is syntactically valid
    pub config_valid: bool,

    /// Config file error message (if invalid)
    pub config_error: Option<String>,

    /// Number of input paths found
    pub paths_found: usize,

    /// Total number of input paths
    pub paths_total: usize,

    /// List of missing paths
    pub missing_paths: Vec<std::path::PathBuf>,

    /// Base URL is accessible
    pub base_url_valid: bool,

    /// Base URL error message (if inaccessible)
    pub base_url_error: Option<String>,

    /// Remote URL validation results
    pub remote_urls_ok: usize,

    /// Total remote URLs checked
    pub remote_urls_total: usize,

    /// Remote URL errors
    pub remote_url_errors: Vec<(String, String)>,
}

impl ValidationResult {
    /// Create a new empty validation result
    pub fn new() -> Self {
        Self {
            config_valid: true, // No config to validate means valid
            config_error: None,
            paths_found: 0,
            paths_total: 0,
            missing_paths: Vec::new(),
            base_url_valid: true, // No URL to validate means valid
            base_url_error: None,
            remote_urls_ok: 0,
            remote_urls_total: 0,
            remote_url_errors: Vec::new(),
        }
    }

    /// Check if all validations passed
    pub fn is_valid(&self) -> bool {
        self.config_valid
            && self.missing_paths.is_empty()
            && (self.base_url_valid || self.base_url_error.is_none())
            && self.remote_url_errors.is_empty()
    }

    /// Get the appropriate exit code
    pub fn exit_code(&self) -> i32 {
        if !self.config_valid {
            return 1; // Config error
        }
        if !self.missing_paths.is_empty() {
            return 2; // Path error
        }
        if self.base_url_error.is_some() || !self.remote_url_errors.is_empty() {
            return 3; // URL error
        }
        0 // Success
    }
}
