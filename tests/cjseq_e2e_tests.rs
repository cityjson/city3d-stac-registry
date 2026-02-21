//! End-to-end tests for CityJSONSequence
use assert_cmd::Command;
use std::io::Write;
use tempfile::tempdir;

#[test]
fn test_cjseq_item_generation() {
    let temp = tempdir().expect("Failed to create temp dir");
    let input_path = temp.path().join("test.jsonl");
    let output_path = temp.path().join("item.json");

    // Create a dummy CJSeq file
    // Line 1: CityJSON header (must be single line)
    // We use serde_json to serialize a struct or just use a single line string
    let header_obj = serde_json::json!({
        "type": "CityJSON",
        "version": "2.0",
        "CityObjects": {},
        "transform": {
            "scale": [0.01, 0.01, 0.01],
            "translate": [0, 0, 0]
        },
        "metadata": {
            "geographicalExtent": [0.0, 0.0, 0.0, 10.0, 10.0, 10.0],
            "referenceSystem": "https://www.opengis.net/def/crs/EPSG/0/7415"
        },
        "vertices": []
    });

    // Line 2: A feature (must be single line)
    let feature_obj = serde_json::json!({
        "type": "CityJSONFeature",
        "id": "feature1",
        "CityObjects": {
            "building1": {
                "type": "Building",
                "geometry": [],
                "attributes": {
                    "measuredHeight": 10.0
                }
            }
        },
        "vertices": []
    });

    let mut file = std::fs::File::create(&input_path).expect("Failed to create input file");
    writeln!(file, "{}", header_obj.to_string()).expect("Failed to write header");
    writeln!(file, "{}", feature_obj.to_string()).expect("Failed to write feature");

    // Run CLI
    let mut cmd = Command::cargo_bin("city3dstac").unwrap();
    cmd.args([
        "item",
        input_path.to_str().unwrap(),
        "-o",
        output_path.to_str().unwrap(),
    ])
    .assert()
    .success();

    // Verify output
    assert!(output_path.exists());
    let content = std::fs::read_to_string(&output_path).expect("Failed to read output");

    // Check for STAC properties
    assert!(content.contains("stac_version"));
    assert!(content.contains("id"));

    // Check for correct projection and metadata
    assert!(content.contains("proj:epsg"));
    assert!(content.contains("7415"));
    assert!(content.contains("city3d:version"));

    // Check that we detected the object (count 1 from feature + 0 from header = 1)
    // Actually header usually doesn't have CityObjects in CJSeq, they are in features.
    // Let's parse JSON to be sure about validation
    let item: serde_json::Value = serde_json::from_str(&content).unwrap();

    assert_eq!(item["properties"]["proj:epsg"], 7415);
    assert_eq!(item["properties"]["city3d:version"], "2.0");
    // Count depends on how aggregation works for CJSeq.
    // The reader should count all city objects across features.
    // "building1" is 1 object.
    assert_eq!(item["properties"]["city3d:city_objects"], 1);
}
