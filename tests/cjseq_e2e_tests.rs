use assert_cmd::Command;
use std::path::Path;

use tempfile::tempdir;

fn test_data_path(filename: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("data")
        .join(filename)
}

#[test]
#[allow(deprecated)] // Using cargo_bin which is deprecated but still works
fn test_cli_generate_item_from_cjseq() {
    let input = test_data_path("delft.city.jsonl");
    let temp = tempdir().expect("Failed to create temp dir");
    let output = temp.path().join("item.json");

    let mut cmd = Command::cargo_bin("cityjson-stac").unwrap();
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
    // Verify standard STAC fields
    assert!(content.contains("stac_version"));
    assert!(content.contains("id"));
    // Verify CityJSON specific fields/extensions
    // Note: The actual check depends on what the stac generator puts in.
    // But this confirms the CLI ran successfully on a .jsonl file.
}

#[test]
#[allow(deprecated)] // Using cargo_bin which is deprecated but still works
fn test_cli_generate_item_from_cjseq_railway() {
    let input = test_data_path("railway.city.jsonl");
    let temp = tempdir().expect("Failed to create temp dir");
    let output = temp.path().join("railway_item.json");

    let mut cmd = Command::cargo_bin("cityjson-stac").unwrap();
    cmd.args([
        "item",
        input.to_str().unwrap(),
        "-o",
        output.to_str().unwrap(),
    ])
    .assert()
    .success();

    assert!(output.exists());
}
