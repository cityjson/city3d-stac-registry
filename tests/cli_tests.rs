//! CLI end-to-end tests

use assert_cmd::Command;
use predicates::prelude::*;
use std::path::Path;
use tempfile::tempdir;

/// Test data directory path
fn test_data_path(filename: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("data")
        .join(filename)
}

mod cli_help_tests {
    use super::*;

    #[test]
    fn test_cli_help() {
        let mut cmd = Command::cargo_bin("city3dstac").unwrap();
        cmd.arg("--help")
            .assert()
            .success()
            .stdout(predicate::str::contains("STAC"))
            .stdout(predicate::str::contains("CityJSON"));
    }

    #[test]
    fn test_cli_version() {
        let mut cmd = Command::cargo_bin("city3dstac").unwrap();
        cmd.arg("--version").assert().success();
    }

    #[test]
    fn test_cli_item_help() {
        let mut cmd = Command::cargo_bin("city3dstac").unwrap();
        cmd.args(["item", "--help"])
            .assert()
            .success()
            .stdout(predicate::str::contains("item"));
    }

    #[test]
    fn test_cli_collection_help() {
        let mut cmd = Command::cargo_bin("city3dstac").unwrap();
        cmd.args(["collection", "--help"])
            .assert()
            .success()
            .stdout(predicate::str::contains("collection"));
    }
}

mod cli_item_tests {
    use super::*;

    #[test]
    fn test_cli_generate_item_to_file() {
        let input = test_data_path("delft.city.json");
        let temp = tempdir().expect("Failed to create temp dir");
        let output = temp.path().join("item.json");

        let mut cmd = Command::cargo_bin("city3dstac").unwrap();
        cmd.args([
            "item",
            input.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
        ])
        .assert()
        .success();

        assert!(output.exists());

        let content = std::fs::read_to_string(&output).expect("Failed to read output");
        assert!(content.contains("stac_version"));
        assert!(content.contains("Feature"));
        // city3d:encoding removed
    }

    #[test]
    fn test_cli_generate_item_success_message() {
        let input = test_data_path("delft.city.json");
        let temp = tempdir().expect("Failed to create temp dir");
        let output = temp.path().join("item.json");

        let mut cmd = Command::cargo_bin("city3dstac").unwrap();
        cmd.args([
            "item",
            input.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stderr(predicate::str::contains("Item written to"));
    }

    #[test]
    fn test_cli_generate_item_with_id() {
        let input = test_data_path("delft.city.json");
        let temp = tempdir().expect("Failed to create temp dir");
        let output = temp.path().join("item.json");

        let mut cmd = Command::cargo_bin("city3dstac").unwrap();
        cmd.args([
            "item",
            input.to_str().unwrap(),
            "--id",
            "custom-id",
            "-o",
            output.to_str().unwrap(),
        ])
        .assert()
        .success();

        // Check the output file contains the custom ID
        let content = std::fs::read_to_string(&output).expect("Failed to read output");
        assert!(content.contains("custom-id"));
    }

    #[test]
    fn test_cli_item_nonexistent_file() {
        let mut cmd = Command::cargo_bin("city3dstac").unwrap();
        cmd.args(["item", "/nonexistent/path/data.json"])
            .assert()
            .failure();
    }

    #[test]
    fn test_cli_item_railway() {
        let input = test_data_path("railway.city.json");
        let temp = tempdir().expect("Failed to create temp dir");
        let output = temp.path().join("railway.json");

        let mut cmd = Command::cargo_bin("city3dstac").unwrap();
        cmd.args([
            "item",
            input.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
        ])
        .assert()
        .success();

        // Check that output contains railway-specific metadata
        let content = std::fs::read_to_string(&output).expect("Failed to read output");
        assert!(content.contains("city3d:city_objects"));
        assert!(content.contains("city3d:lods"));
    }
}
