use city3d_stac::config::CatalogConfigFile;
use city3d_stac::stac::StacCatalogBuilder;
use std::io::Write;
use tempfile::tempdir;

#[test]
fn test_catalog_config_from_toml() {
    let toml_content = r#"
        id = "test-catalog"
        title = "Test Catalog"
        description = "A test catalog"
        collections = ["col1", "col2"]
    "#;

    let dir = tempdir().unwrap();
    let file_path = dir.path().join("catalog.toml");
    let mut file = std::fs::File::create(&file_path).unwrap();
    file.write_all(toml_content.as_bytes()).unwrap();

    let config = CatalogConfigFile::from_file(&file_path).unwrap();

    assert_eq!(config.id, Some("test-catalog".to_string()));
    assert_eq!(config.title, Some("Test Catalog".to_string()));
    assert_eq!(config.description, Some("A test catalog".to_string()));
    assert_eq!(
        config.collections,
        Some(vec!["col1".to_string(), "col2".to_string()])
    );
}

#[test]
fn test_catalog_builder() {
    let catalog = StacCatalogBuilder::new("cat-id", "cat-desc")
        .title("My Catalog")
        .child_link("./child/collection.json", Some("Child Title".to_string()))
        .self_link("./catalog.json")
        .build();

    assert_eq!(catalog.id, "cat-id");
    assert_eq!(catalog.description, "cat-desc");
    assert_eq!(catalog.title, Some("My Catalog".to_string()));
    assert_eq!(catalog.links.len(), 2);

    let child_link = catalog.links.iter().find(|l| l.rel == "child").unwrap();
    assert_eq!(child_link.href, "./child/collection.json");
    assert_eq!(child_link.title, Some("Child Title".to_string()));
    assert_eq!(child_link.link_type, Some("application/json".to_string()));

    let self_link = catalog.links.iter().find(|l| l.rel == "self").unwrap();
    assert_eq!(self_link.href, "./catalog.json");
}

#[test]
fn test_catalog_serialization() {
    let catalog = StacCatalogBuilder::new("cat-json", "serialization test").build();
    let json = serde_json::to_string(&catalog).unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed["type"], "Catalog");
    assert_eq!(parsed["stac_version"], "1.0.0");
    assert_eq!(parsed["id"], "cat-json");
    assert_eq!(parsed["description"], "serialization test");
    assert!(parsed.get("extent").is_none()); // Catalog shouldn't have extent
    assert!(parsed.get("license").is_none()); // Catalog shouldn't have license
}

#[test]
fn test_cli_catalog_command() {
    use assert_cmd::Command;

    let dir = tempdir().unwrap();
    let data_dir = dir.path().join("data");
    std::fs::create_dir(&data_dir).unwrap();

    // Create a dummy CityJSON file
    let cityjson_content = r#"{
        "type": "CityJSON",
        "version": "1.1",
        "CityObjects": {},
        "vertices": [],
        "transform": {
            "scale": [0.001, 0.001, 0.001],
            "translate": [0.0, 0.0, 0.0]
        }
    }"#;
    std::fs::write(data_dir.join("test.city.json"), cityjson_content).unwrap();

    let output_dir = dir.path().join("catalog");

    // Run catalog command
    let mut cmd = Command::cargo_bin("city3dstac").unwrap();
    cmd.args([
        "catalog",
        data_dir.to_str().unwrap(),
        "-o",
        output_dir.to_str().unwrap(),
        "--id",
        "test-catalog",
        "--description",
        "Test Catalog",
    ])
    .assert()
    .success();

    // Verify catalog.json exists
    let catalog_path = output_dir.join("catalog.json");
    assert!(catalog_path.exists());

    let catalog_content = std::fs::read_to_string(&catalog_path).unwrap();
    let catalog_json: serde_json::Value = serde_json::from_str(&catalog_content).unwrap();

    assert_eq!(catalog_json["id"], "test-catalog");
    assert_eq!(catalog_json["description"], "Test Catalog");

    // Verify sub-collection exists
    // The collection directory name should be "data" (from the input directory name)
    let collection_dir = output_dir.join("data");
    assert!(collection_dir.exists());

    let collection_path = collection_dir.join("collection.json");
    assert!(collection_path.exists());
}
