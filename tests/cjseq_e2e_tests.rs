use assert_cmd::Command;
use std::path::Path;

use tempfile::tempdir;

fn test_data_path(filename: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("data")
        .join(filename)
}

// TODO: implement tests
