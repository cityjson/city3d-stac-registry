//! CLI end-to-end tests

#![allow(deprecated)] // Command::cargo_bin is deprecated but still functional

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

/// Get the tests/data directory
fn test_data_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("data")
}

mod cli_help_tests {
    use super::*;

    #[test]
    fn test_cli_help() {
        let mut cmd = Command::cargo_bin("cjstac").unwrap();
        cmd.arg("--help")
            .assert()
            .success()
            .stdout(predicate::str::contains("STAC"))
            .stdout(predicate::str::contains("CityJSON"));
    }

    #[test]
    fn test_cli_version() {
        let mut cmd = Command::cargo_bin("cjstac").unwrap();
        cmd.arg("--version").assert().success();
    }

    #[test]
    fn test_cli_item_help() {
        let mut cmd = Command::cargo_bin("cjstac").unwrap();
        cmd.args(["item", "--help"])
            .assert()
            .success()
            .stdout(predicate::str::contains("item"));
    }

    #[test]
    fn test_cli_collection_help() {
        let mut cmd = Command::cargo_bin("cjstac").unwrap();
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

        let mut cmd = Command::cargo_bin("cjstac").unwrap();
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
        assert!(content.contains("city3d:encoding"));
    }

    #[test]
    fn test_cli_generate_item_success_message() {
        let input = test_data_path("delft.city.json");
        let temp = tempdir().expect("Failed to create temp dir");
        let output = temp.path().join("item.json");

        let mut cmd = Command::cargo_bin("cjstac").unwrap();
        cmd.args([
            "item",
            input.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Generated STAC Item"));
    }

    #[test]
    fn test_cli_generate_item_with_id() {
        let input = test_data_path("delft.city.json");
        let temp = tempdir().expect("Failed to create temp dir");
        let output = temp.path().join("item.json");

        let mut cmd = Command::cargo_bin("cjstac").unwrap();
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
        let mut cmd = Command::cargo_bin("cjstac").unwrap();
        cmd.args(["item", "/nonexistent/path/data.json"])
            .assert()
            .failure();
    }

    #[test]
    fn test_cli_item_railway() {
        let input = test_data_path("railway.city.json");
        let temp = tempdir().expect("Failed to create temp dir");
        let output = temp.path().join("railway.json");

        let mut cmd = Command::cargo_bin("cjstac").unwrap();
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

mod cli_collection_tests {
    use super::*;

    #[test]
    fn test_cli_generate_collection_from_directory() {
        let input = test_data_dir();
        let temp = tempdir().expect("Failed to create temp dir");
        let output_dir = temp.path().join("output");

        let mut cmd = Command::cargo_bin("cjstac").unwrap();
        cmd.args([
            "collection",
            input.to_str().unwrap(),
            "--id",
            "test-collection",
            "-o",
            output_dir.to_str().unwrap(),
        ])
        .assert()
        .success();

        // The collection command creates a directory with collection.json inside
        let collection_file = output_dir.join("collection.json");
        assert!(
            collection_file.exists(),
            "collection.json should exist in output directory"
        );

        let content = std::fs::read_to_string(&collection_file).expect("Failed to read output");
        assert!(content.contains("Collection"));
        assert!(content.contains("test-collection"));
    }

    #[test]
    fn test_cli_generate_collection_success_message() {
        let input = test_data_dir();
        let temp = tempdir().expect("Failed to create temp dir");
        let output_dir = temp.path().join("output");

        let mut cmd = Command::cargo_bin("cjstac").unwrap();
        cmd.args([
            "collection",
            input.to_str().unwrap(),
            "--id",
            "test",
            "-o",
            output_dir.to_str().unwrap(),
        ])
        .assert()
        .success()
        // The collection command prints "Generated X items" and item/collection paths
        .stdout(predicate::str::contains("Generated"));
    }

    #[test]
    fn test_cli_generate_collection_with_title() {
        let input = test_data_dir();
        let temp = tempdir().expect("Failed to create temp dir");
        let output_dir = temp.path().join("output");

        let mut cmd = Command::cargo_bin("cjstac").unwrap();
        cmd.args([
            "collection",
            input.to_str().unwrap(),
            "--id",
            "test",
            "--title",
            "My Custom Title",
            "-o",
            output_dir.to_str().unwrap(),
        ])
        .assert()
        .success();

        let collection_file = output_dir.join("collection.json");
        let content = std::fs::read_to_string(&collection_file).expect("Failed to read output");
        assert!(content.contains("My Custom Title"));
    }

    #[test]
    fn test_cli_generate_collection_with_description() {
        let input = test_data_dir();
        let temp = tempdir().expect("Failed to create temp dir");
        let output_dir = temp.path().join("output");

        let mut cmd = Command::cargo_bin("cjstac").unwrap();
        cmd.args([
            "collection",
            input.to_str().unwrap(),
            "--id",
            "test",
            "--description",
            "A test collection for CityJSON data",
            "-o",
            output_dir.to_str().unwrap(),
        ])
        .assert()
        .success();

        let collection_file = output_dir.join("collection.json");
        let content = std::fs::read_to_string(&collection_file).expect("Failed to read output");
        assert!(content.contains("A test collection"));
    }

    #[test]
    fn test_cli_collection_nonexistent_path() {
        let mut cmd = Command::cargo_bin("cjstac").unwrap();
        cmd.args(["collection", "/nonexistent/path", "--id", "test"])
            .assert()
            .failure();
    }

    #[test]
    fn test_cli_collection_single_file_error() {
        // Collection command now supports individual files as inputs
        let input = test_data_path("delft.city.json");
        let temp = tempdir().expect("Failed to create temp dir");
        let output = temp.path().join("stac_output");

        let mut cmd = Command::cargo_bin("cjstac").unwrap();
        cmd.args([
            "collection",
            input.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
            "--id",
            "test",
        ])
        .assert()
        .success()
        .stderr(predicate::str::contains("Scanning 1 input(s)"));
    }
}

mod cli_verbose_tests {
    use super::*;

    #[test]
    fn test_cli_verbose_mode() {
        let input = test_data_path("delft.city.json");
        let temp = tempdir().expect("Failed to create temp dir");
        let output = temp.path().join("item.json");

        let mut cmd = Command::cargo_bin("cjstac").unwrap();
        cmd.args([
            "--verbose",
            "item",
            input.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
        ])
        .assert()
        .success();
    }
}

mod cli_edge_cases {
    use super::*;

    #[test]
    fn test_cli_no_subcommand() {
        let mut cmd = Command::cargo_bin("cjstac").unwrap();
        cmd.assert()
            .failure()
            .stderr(predicate::str::contains("Usage"));
    }

    #[test]
    fn test_cli_unknown_subcommand() {
        let mut cmd = Command::cargo_bin("cjstac").unwrap();
        cmd.arg("unknown").assert().failure();
    }
}
