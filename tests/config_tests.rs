//! Tests for YAML configuration functionality

use std::io::Write;
use tempfile::NamedTempFile;

/// Test that a valid YAML config file can be parsed
#[test]
fn test_config_file_parsing() {
    let yaml_content = r#"
id: test-collection
title: Test Collection
description: |
  A test collection
  with multiple lines
license: CC-BY-4.0
keywords:
  - test
  - cityjson
  - 3d
providers:
  - name: Test Provider
    url: https://example.com
    roles:
      - producer
      - licensor
    description: A test provider
"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    writeln!(temp_file, "{}", yaml_content).unwrap();
    temp_file.flush().unwrap();

    // Parse the config file
    let config = city3d_stac::config::CollectionConfigFile::from_file(temp_file.path())
        .expect("Failed to parse config file");

    assert_eq!(config.id, Some("test-collection".to_string()));
    assert_eq!(config.title, Some("Test Collection".to_string()));
    // YAML multiline literals preserve the trailing newline
    assert_eq!(
        config.description,
        Some("A test collection\nwith multiple lines\n".to_string())
    );
    assert_eq!(config.license, Some("CC-BY-4.0".to_string()));
    assert_eq!(
        config.keywords,
        Some(vec![
            "test".to_string(),
            "cityjson".to_string(),
            "3d".to_string()
        ])
    );

    // Check providers
    let providers = config.providers.expect("No providers found");
    assert_eq!(providers.len(), 1);
    assert_eq!(providers[0].name, "Test Provider");
    assert_eq!(providers[0].url, Some("https://example.com".to_string()));
    assert_eq!(
        providers[0].roles,
        Some(vec!["producer".to_string(), "licensor".to_string()])
    );
    assert_eq!(
        providers[0].description,
        Some("A test provider".to_string())
    );
}

/// Test that invalid YAML produces an error
#[test]
fn test_invalid_yaml() {
    let invalid_yaml = r#"
id: test-collection
title: Test Collection
invalid: [unclosed list
"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    writeln!(temp_file, "{}", invalid_yaml).unwrap();
    temp_file.flush().unwrap();

    let result = city3d_stac::config::CollectionConfigFile::from_file(temp_file.path());

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("Invalid YAML"));
}

/// Test that CLI arguments override config file values
#[test]
fn test_config_cli_merge() {
    use city3d_stac::config::{CollectionCliArgs, CollectionConfigFile};

    let yaml_content = r#"
id: from-file
title: File Title
description: File Description
license: Apache-2.0
keywords:
  - tag1
  - tag2
"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    writeln!(temp_file, "{}", yaml_content).unwrap();
    temp_file.flush().unwrap();

    let file_config =
        CollectionConfigFile::from_file(temp_file.path()).expect("Failed to parse config file");

    let cli_args = CollectionCliArgs {
        id: Some("from-cli".to_string()),
        title: Some("CLI Title".to_string()),
        description: None, // Keep from file
        license: Some("MIT".to_string()),
        base_url: None,
    };

    let merged = file_config.merge_with_cli(&cli_args);

    // CLI args should override for id, title, license
    assert_eq!(merged.id, Some("from-cli".to_string()));
    assert_eq!(merged.title, Some("CLI Title".to_string()));
    assert_eq!(merged.license, Some("MIT".to_string()));

    // File config should be preserved for description, keywords
    assert_eq!(merged.description, Some("File Description".to_string()));
    assert_eq!(
        merged.keywords,
        Some(vec!["tag1".to_string(), "tag2".to_string()])
    );
}

/// Test minimal config (all optional fields omitted)
#[test]
fn test_minimal_config() {
    let yaml_content = r#"
id: minimal-collection
"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    writeln!(temp_file, "{}", yaml_content).unwrap();
    temp_file.flush().unwrap();

    let config = city3d_stac::config::CollectionConfigFile::from_file(temp_file.path())
        .expect("Failed to parse config file");

    assert_eq!(config.id, Some("minimal-collection".to_string()));
    assert_eq!(config.title, None);
    assert_eq!(config.description, None);
    assert_eq!(config.license, None);
    assert_eq!(config.keywords, None);
    assert_eq!(config.providers, None);
}

/// Test config with extent configuration
#[test]
fn test_config_with_extent() {
    let yaml_content = r#"
id: test-with-extent
extent:
  spatial:
    bbox: [4.42, 51.88, 0.0, 4.6, 51.98, 100.0]
    crs: EPSG:7415
  temporal:
    start: "2023-01-01T00:00:00Z"
    end: null
"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    writeln!(temp_file, "{}", yaml_content).unwrap();
    temp_file.flush().unwrap();

    let config = city3d_stac::config::CollectionConfigFile::from_file(temp_file.path())
        .expect("Failed to parse config file");

    assert!(config.extent.is_some());
    let extent = config.extent.unwrap();
    assert!(extent.spatial.is_some());
    let spatial = extent.spatial.unwrap();
    assert_eq!(
        spatial.bbox,
        Some(vec![4.42, 51.88, 0.0, 4.6, 51.98, 100.0])
    );
    assert_eq!(spatial.crs, Some("EPSG:7415".to_string()));

    assert!(extent.temporal.is_some());
    let temporal = extent.temporal.unwrap();
    assert_eq!(temporal.start, Some("2023-01-01T00:00:00Z".to_string()));
    assert_eq!(temporal.end, None);
}

/// Test config with links configuration
#[test]
fn test_config_with_links() {
    let yaml_content = r#"
id: test-with-links
links:
  - rel: license
    href: https://creativecommons.org/licenses/by/4.0/
    type: text/html
    title: CC-BY-4.0 License
  - rel: about
    href: https://example.com
    title: Project Homepage
"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    writeln!(temp_file, "{}", yaml_content).unwrap();
    temp_file.flush().unwrap();

    let config = city3d_stac::config::CollectionConfigFile::from_file(temp_file.path())
        .expect("Failed to parse config file");

    assert!(config.links.is_some());
    let links = config.links.unwrap();
    assert_eq!(links.len(), 2);

    assert_eq!(links[0].rel, "license");
    assert_eq!(
        links[0].href,
        "https://creativecommons.org/licenses/by/4.0/"
    );
    assert_eq!(links[0].link_type, Some("text/html".to_string()));
    assert_eq!(links[0].title, Some("CC-BY-4.0 License".to_string()));

    assert_eq!(links[1].rel, "about");
    assert_eq!(links[1].href, "https://example.com");
    assert_eq!(links[1].link_type, None);
    assert_eq!(links[1].title, Some("Project Homepage".to_string()));
}

/// Test provider conversion from config to STAC model
#[test]
fn test_provider_conversion() {
    use city3d_stac::config::ProviderConfig;
    use city3d_stac::stac::Provider;

    let config_provider = ProviderConfig {
        name: "Test Provider".to_string(),
        url: Some("https://example.com".to_string()),
        roles: Some(vec!["producer".to_string(), "licensor".to_string()]),
        description: Some("A test provider".to_string()),
    };

    let stac_provider: Provider = config_provider.into();

    assert_eq!(stac_provider.name, "Test Provider");
    assert_eq!(stac_provider.url, Some("https://example.com".to_string()));
    assert_eq!(
        stac_provider.roles,
        Some(vec!["producer".to_string(), "licensor".to_string()])
    );
    assert_eq!(
        stac_provider.description,
        Some("A test provider".to_string())
    );
}

/// Test config with custom summaries
#[test]
fn test_config_with_summaries() {
    let yaml_content = r#"
id: test-with-summaries
summaries:
  city3d:encoding:
    - CityJSON
    - CityGML
  custom:field: custom value
  eo:cloud_cover: 5
"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    writeln!(temp_file, "{}", yaml_content).unwrap();
    temp_file.flush().unwrap();

    let config = city3d_stac::config::CollectionConfigFile::from_file(temp_file.path())
        .expect("Failed to parse config file");

    assert!(config.summaries.is_some());
    let summaries = config.summaries.unwrap();
    assert_eq!(summaries.len(), 3);
    assert!(summaries.contains_key("city3d:encoding"));
    assert!(summaries.contains_key("custom:field"));
    assert!(summaries.contains_key("eo:cloud_cover"));
}

/// Test config with inputs field
#[test]
fn test_config_with_inputs() {
    use city3d_stac::config::InputsConfig;

    let yaml_content = r#"
id: test-with-inputs
inputs:
  - "file1.json"
  - "file2.json"
  - "data/*.city.json"
  - "/absolute/path/to/file.jsonl"
"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    writeln!(temp_file, "{}", yaml_content).unwrap();
    temp_file.flush().unwrap();

    let config = city3d_stac::config::CollectionConfigFile::from_file(temp_file.path())
        .expect("Failed to parse config file");

    assert!(config.inputs.is_some());
    let inputs = config.inputs.unwrap();
    match inputs {
        InputsConfig::Inline(urls) => {
            assert_eq!(urls.len(), 4);
            assert_eq!(urls[0], "file1.json");
            assert_eq!(urls[1], "file2.json");
            assert_eq!(urls[2], "data/*.city.json");
            assert_eq!(urls[3], "/absolute/path/to/file.jsonl");
        }
        InputsConfig::FromFile { from_file } => {
            panic!("Expected Inline inputs, got FromFile: {}", from_file);
        }
    }
}

/// Test config with from_file inputs
#[test]
fn test_config_with_inputs_from_file() {
    use city3d_stac::config::InputsConfig;
    use std::io::Write;

    // Create a temp file with URLs
    let mut urls_file = NamedTempFile::new().unwrap();
    writeln!(urls_file, "https://example.com/file1.json").unwrap();
    writeln!(urls_file, "https://example.com/file2.json").unwrap();
    writeln!(urls_file, "# This is a comment").unwrap();
    writeln!(urls_file, "").unwrap(); // Empty line
    writeln!(urls_file, "https://example.com/file3.json").unwrap();
    urls_file.flush().unwrap();

    let urls_path = urls_file.path();

    // Create config file that references the URLs file
    let yaml_content = format!(
        r#"
id: test-from-file
inputs:
  from_file: {}
"#,
        urls_path.display()
    );

    let mut config_file = NamedTempFile::new().unwrap();
    writeln!(config_file, "{}", yaml_content).unwrap();
    config_file.flush().unwrap();

    let config = city3d_stac::config::CollectionConfigFile::from_file(config_file.path())
        .expect("Failed to parse config file");

    assert!(config.inputs.is_some());
    let inputs = config.inputs.unwrap();

    match inputs {
        InputsConfig::Inline(urls) => {
            panic!(
                "Expected FromFile inputs, got Inline with {} URLs",
                urls.len()
            );
        }
        InputsConfig::FromFile { ref from_file } => {
            assert_eq!(from_file, &urls_path.display().to_string());
        }
    }

    // Test resolve() method
    let config_dir = config_file.path().parent().unwrap();
    let resolved = inputs
        .resolve(config_dir)
        .expect("Failed to resolve inputs");

    // Should have 3 URLs (comment and empty line filtered out)
    assert_eq!(resolved.len(), 3);
    assert_eq!(resolved[0], "https://example.com/file1.json");
    assert_eq!(resolved[1], "https://example.com/file2.json");
    assert_eq!(resolved[2], "https://example.com/file3.json");
}
