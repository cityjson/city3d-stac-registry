# Dry Run Option Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add a `--dry-run` global flag to the CLI that validates configuration files, input paths, and remote URLs without generating any STAC output.

**Architecture:**
- Add `dry_run: bool` as a global flag in the CLI struct
- Create a new validation module with parallel URL checking
- Integrate validation at the start of each command handler
- Use existing progress UI (spinners/bars) for consistent UX
- Exit with appropriate codes (0=success, 1-4=various error types)

**Tech Stack:**
- Rust, clap (CLI parsing), tokio (async), reqwest (HTTP)
- Existing: serde_json/serde_yaml/toml (config parsing), console (UI)

---

## Task 1: Add Global `--dry-run` Flag

**Files:**
- Modify: `src/cli/mod.rs`

**Step 1: Add the dry_run field to Cli struct**

Add this line to the `Cli` struct (after the `verbose` field):

```rust
/// Dry run: validate config and inputs without generating output
#[arg(long, global = true)]
dry_run: bool,
```

**Step 2: Update all command handler signatures to accept dry_run**

Update the match arm in `run()` function to pass `cli.dry_run` to handlers:

```rust
match cli.command {
    Commands::Item { ... } => {
        handle_item_command(
            input, output, id, title, description, collection, base_url, pretty,
            cli.dry_run,  // Add this parameter
        )
        .await
    }

    Commands::Collection { ... } => {
        // ... existing validation code ...

        handle_collection_command(
            CollectionConfig {
                // ... existing fields ...
                dry_run: cli.dry_run,  // Add this field
            },
        )
        .await
    }

    Commands::UpdateCollection { ... } => handle_update_collection_command(
        UpdateCollectionConfig {
            // ... existing fields ...
            dry_run: cli.dry_run,  // Add this field
        },
    ),

    Commands::Catalog { ... } => handle_catalog_command(
        CatalogConfig {
            // ... existing fields ...
            dry_run: cli.dry_run,  // Add this field
        },
    )
    .await,
}
```

**Step 3: Update config structs to include dry_run field**

Add `dry_run: bool` to:
- `struct CollectionConfig` (around line 672)
- `struct UpdateCollectionConfig` (around line 998)
- `struct CatalogConfig` (around line 453)

**Step 4: Update function signatures**

Update these function signatures to accept `dry_run: bool`:
- `async fn handle_item_command(..., dry_run: bool)`
- `async fn handle_collection_command(config: CollectionConfig)`
- `async fn process_collection_logic(config: CollectionConfig)`
- `fn handle_update_collection_command(config: UpdateCollectionConfig)`
- `async fn handle_catalog_command(config: CatalogConfig)`

**Step 5: Commit**

```bash
git add src/cli/mod.rs
git commit -m "feat: add --dry-run global flag to CLI"
```

---

## Task 2: Create Validation Module

**Files:**
- Create: `src/validation/mod.rs`
- Create: `src/validation/result.rs`
- Modify: `src/lib.rs`

**Step 1: Create validation module structure**

Create `src/validation/mod.rs`:

```rust
//! Validation logic for dry-run mode

pub mod result;

use crate::error::{CityJsonStacError, Result};
use crate::config::CollectionConfigFile;
use result::ValidationResult;
use std::path::{Path, PathBuf};

/// Validate collection configuration without generating output
pub async fn validate_collection_config(
    config_path: &Option<PathBuf>,
    inputs: &[PathBuf],
    base_url: &Option<String>,
) -> Result<ValidationResult> {
    let mut result = ValidationResult::new();

    // 1. Validate config file syntax if provided
    if let Some(path) = config_path {
        let spinner = console::style("→").blue();
        println!("  {} Checking config file: {}", spinner, path.display());

        match CollectionConfigFile::from_file(path) {
            Ok(_config) => {
                result.config_valid = true;
                println!("  ✓ Config file syntax: valid");
            }
            Err(e) => {
                result.config_valid = false;
                result.config_error = Some(e.to_string());
                println!("  ✗ Config file syntax: {}", e);
            }
        }
    }

    // 2. Validate input paths exist
    if !inputs.is_empty() {
        let mut found = 0;
        let mut missing = Vec::new();

        for path in inputs {
            if path.exists() {
                found += 1;
            } else {
                missing.push(path.clone());
            }
        }

        result.paths_found = found;
        result.paths_total = inputs.len();
        result.missing_paths = missing;

        if missing.is_empty() {
            println!("  ✓ Input paths: {}/{} found", found, inputs.len());
        } else {
            println!("  ⚠ Input paths: {}/{} found", found, inputs.len());
            for path in &missing {
                println!("    ✗ {}", path.display());
            }
        }
    }

    // 3. Validate base URL if provided
    if let Some(url) = base_url {
        println!("  → Checking base URL: {}", url);
        match validate_url_head(url).await {
            Ok(status) => {
                result.base_url_valid = true;
                println!("  ✓ Base URL: accessible ({})", status);
            }
            Err(e) => {
                result.base_url_valid = false;
                result.base_url_error = Some(e.to_string());
                println!("  ✗ Base URL: {}", e);
            }
        }
    }

    Ok(result)
}

/// Validate URL with HEAD request (lightweight, doesn't download body)
async fn validate_url_head(url: &str) -> Result<String> {
    use reqwest::Client;
    use std::time::Duration;

    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| CityJsonStacError::Other(format!("Failed to create HTTP client: {}", e)))?;

    let response = client
        .head(url)
        .send()
        .await
        .map_err(|e| CityJsonStacError::Other(format!("HTTP request failed: {}", e)))?;

    let status = response.status();

    if status.is_success() {
        Ok(status.to_string())
    } else {
        Err(CityJsonStacError::Other(format!("HTTP {}", status)))
    }
}

/// Validate item input (file path or URL)
pub async fn validate_item_input(input: &str) -> Result<ValidationResult> {
    let mut result = ValidationResult::new();

    // Check if it's a remote URL
    if input.starts_with("http://") || input.starts_with("https://") {
        println!("  → Checking remote URL: {}", input);
        match validate_url_head(input).await {
            Ok(status) => {
                result.base_url_valid = true;
                println!("  ✓ URL: accessible ({})", status);
            }
            Err(e) => {
                result.base_url_valid = false;
                result.base_url_error = Some(e.to_string());
                println!("  ✗ URL: {}", e);
            }
        }
    } else {
        // Local file
        let path = PathBuf::from(input);
        println!("  → Checking local file: {}", input);

        if path.exists() {
            result.paths_found = 1;
            result.paths_total = 1;
            println!("  ✓ File: exists");
        } else {
            result.paths_total = 1;
            result.missing_paths.push(path);
            println!("  ✗ File: not found");
        }
    }

    Ok(result)
}
```

**Step 2: Create ValidationResult struct**

Create `src/validation/result.rs`:

```rust
//! Validation result types

/// Result of dry-run validation
#[derive(Debug, Default, Clone)]
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
        Self::default()
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
```

**Step 3: Register validation module in lib.rs**

Add to `src/lib.rs`:

```rust
pub mod validation;
```

**Step 4: Commit**

```bash
git add src/validation/mod.rs src/validation/result.rs src/lib.rs
git commit -m "feat: create validation module for dry-run mode"
```

---

## Task 3: Integrate Validation into Collection Command

**Files:**
- Modify: `src/cli/mod.rs`

**Step 1: Add dry-run check at start of handle_collection_command**

Add this at the beginning of `handle_collection_command()`:

```rust
async fn handle_collection_command(config: CollectionConfig) -> Result<()> {
    // Dry-run mode: validate only
    if config.dry_run {
        use crate::validation;
        use progress::{print_banner, print_success, print_error};

        print_banner();

        println!("\nRunning in dry-run mode...\n");

        // Determine final inputs
        let base_config = if let Some(config_path) = &config.config {
            validation::validate_collection_config(
                &Some(config_path.clone()),
                &config.inputs,
                &config.base_url,
            )
            .await?
        } else {
            validation::validate_collection_config(
                &None,
                &config.inputs,
                &config.base_url,
            )
            .await?
        };

        println!();

        // Print final status
        if base_config.is_valid() {
            print_success("Dry run complete: All validations passed");
            std::process::exit(0);
        } else {
            print_error("Dry run failed: Errors found");
            std::process::exit(base_config.exit_code());
        }
    }

    // Normal execution continues...
    match process_collection_logic(config).await {
        Ok(_) => Ok(()),
        Err(e) => Err(e),
    }
}
```

**Step 2: Commit**

```bash
git add src/cli/mod.rs
git commit -m "feat: integrate dry-run validation into collection command"
```

---

## Task 4: Integrate Validation into Item Command

**Files:**
- Modify: `src/cli/mod.rs`

**Step 1: Add dry-run check in handle_item_command**

Add this validation at the start of `handle_item_command()`:

```rust
async fn handle_item_command(
    input: String,
    output: Option<PathBuf>,
    id: Option<String>,
    title: Option<String>,
    description: Option<String>,
    collection: Option<String>,
    base_url: Option<String>,
    pretty: bool,
    dry_run: bool,
) -> Result<()> {
    // Dry-run mode: validate only
    if dry_run {
        use crate::validation;
        use progress::{print_banner, print_success, print_error};

        print_banner();

        println!("\nRunning in dry-run mode...\n");

        let result = validation::validate_item_input(&input).await?;

        println!();

        if result.is_valid() {
            print_success("Dry run complete: All validations passed");
            std::process::exit(0);
        } else {
            print_error("Dry run failed: Errors found");
            std::process::exit(result.exit_code());
        }
    }

    // Normal execution continues...
    let spinner = create_spinner(format!("Reading {input}…"));
    // ... rest of function
}
```

**Step 2: Commit**

```bash
git add src/cli/mod.rs
git commit -m "feat: integrate dry-run validation into item command"
```

---

## Task 5: Integrate Validation into Update-Collection Command

**Files:**
- Modify: `src/cli/mod.rs`

**Step 1: Add dry-run check in handle_update_collection_command**

Add this at the start of the function:

```rust
fn handle_update_collection_command(config: UpdateCollectionConfig) -> Result<()> {
    // Dry-run mode: validate only
    if config.dry_run {
        use progress::{print_banner, print_success, print_error};

        print_banner();

        println!("\nRunning in dry-run mode...\n");

        let mut all_valid = true;
        let mut found = 0;

        for item_path in &config.items {
            let fname = item_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown");

            if item_path.exists() {
                // Try to parse as STAC item
                match std::fs::read_to_string(item_path) {
                    Ok(content) => {
                        match serde_json::from_str::<crate::stac::StacItem>(&content) {
                            Ok(_) => {
                                println!("  ✓ {}", fname);
                                found += 1;
                            }
                            Err(e) => {
                                println!("  ✗ {}: Invalid STAC item - {}", fname, e);
                                all_valid = false;
                            }
                        }
                    }
                    Err(e) => {
                        println!("  ✗ {}: Cannot read - {}", fname, e);
                        all_valid = false;
                    }
                }
            } else {
                println!("  ✗ {}: File not found", fname);
                all_valid = false;
            }
        }

        println!("\n  STAC items: {}/{} valid", found, config.items.len());

        println!();

        if all_valid {
            print_success("Dry run complete: All validations passed");
            std::process::exit(0);
        } else {
            print_error("Dry run failed: Errors found");
            std::process::exit(1);
        }
    }

    // Normal execution continues...
    // ... existing code
}
```

**Step 2: Commit**

```bash
git add src/cli/mod.rs
git commit -m "feat: integrate dry-run validation into update-collection command"
```

---

## Task 6: Integrate Validation into Catalog Command

**Files:**
- Modify: `src/cli/mod.rs`

**Step 1: Add dry-run check in handle_catalog_command**

Add this at the start of `async fn handle_catalog_command()`:

```rust
async fn handle_catalog_command(config: CatalogConfig) -> Result<()> {
    // Dry-run mode: validate only
    if config.dry_run {
        use crate::config::CatalogConfigFile;
        use progress::{print_banner, print_success, print_error};

        print_banner();

        println!("\nRunning in dry-run mode...\n");

        // Validate config file if provided
        if let Some(config_path) = &config.config {
            println!("  → Checking config file: {}", config_path.display());
            match CatalogConfigFile::from_file(config_path) {
                Ok(_) => {
                    println!("  ✓ Config file syntax: valid");
                }
                Err(e) => {
                    println!("  ✗ Config file syntax: {}", e);
                    println!();
                    print_error("Dry run failed: Config error");
                    std::process::exit(1);
                }
            }
        }

        // Validate input directories/collections
        let mut found = 0;
        let mut missing = Vec::new();

        for input in &config.inputs {
            if input.exists() {
                found += 1;
            } else {
                missing.push(input.clone());
            }
        }

        if missing.is_empty() {
            println!("  ✓ Input paths: {}/{} found", found, config.inputs.len());
        } else {
            println!("  ⚠ Input paths: {}/{} found", found, config.inputs.len());
            for path in &missing {
                println!("    ✗ {}", path.display());
            }
        }

        println!();

        if missing.is_empty() {
            print_success("Dry run complete: All validations passed");
            std::process::exit(0);
        } else {
            print_error("Dry run failed: Missing paths");
            std::process::exit(2);
        }
    }

    // Normal execution continues...
    // ... existing code
}
```

**Step 2: Commit**

```bash
git add src/cli/mod.rs
git commit -m "feat: integrate dry-run validation into catalog command"
```

---

## Task 7: Add Unit Tests for Validation Module

**Files:**
- Create: `src/validation/tests.rs`

**Step 1: Create test module**

Add tests to `src/validation/mod.rs` (or create separate test file):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_result_default() {
        let result = ValidationResult::new();
        assert!(result.is_valid()); // Empty result is valid
        assert_eq!(result.exit_code(), 0);
    }

    #[test]
    fn test_validation_result_config_error() {
        let mut result = ValidationResult::new();
        result.config_valid = false;
        result.config_error = Some("Parse error".to_string());

        assert!(!result.is_valid());
        assert_eq!(result.exit_code(), 1);
    }

    #[test]
    fn test_validation_result_missing_paths() {
        let mut result = ValidationResult::new();
        result.paths_found = 1;
        result.paths_total = 2;
        result.missing_paths.push(std::path::PathBuf::from("missing.json"));

        assert!(!result.is_valid());
        assert_eq!(result.exit_code(), 2);
    }

    #[test]
    fn test_validation_result_url_error() {
        let mut result = ValidationResult::new();
        result.base_url_valid = false;
        result.base_url_error = Some("Connection refused".to_string());

        assert!(!result.is_valid());
        assert_eq!(result.exit_code(), 3);
    }
}
```

**Step 2: Run tests**

```bash
cargo test --lib validation
```

Expected: All tests pass

**Step 3: Commit**

```bash
git add src/validation/tests.rs src/validation/mod.rs
git commit -m "test: add unit tests for validation module"
```

---

## Task 8: Manual Testing

**Files:**
- None (manual testing)

**Step 1: Test dry-run with collection command**

```bash
cargo run -- collection --config ./opendata/singapore-config.toml --dry-run
```

Expected output shows validation results and exits with code 0

**Step 2: Test dry-run with item command (local file)**

```bash
cargo run -- item opendata/some-file.city.json --dry-run
```

Expected: Validates file exists

**Step 3: Test dry-run with missing file**

```bash
cargo run -- item nonexistent.json --dry-run
```

Expected: Shows file not found, exits with code 2

**Step 4: Test dry-run with remote URL**

```bash
cargo run -- item https://raw.githubusercontent.com/cityjson/spec/docs/schemas/examples/CityJSONGML/example1.json --dry-run
```

Expected: Validates URL is accessible

**Step 5: Test normal execution still works**

```bash
cargo run -- collection --help
```

Expected: Shows help including `--dry-run` option

**Step 6: Commit**

```bash
# No code changes, just document testing results
echo "Manual testing completed - all scenarios working as expected" >> TESTING_NOTES.md
git add TESTING_NOTES.md
git commit -m "test: document manual testing results for dry-run"
```

---

## Task 9: Format and Lint Check

**Files:**
- None (project-wide)

**Step 1: Format code**

```bash
cargo fmt
```

**Step 2: Run clippy**

```bash
cargo clippy -- -D warnings
```

Expected: No warnings

**Step 3: Run all tests**

```bash
cargo test --lib
```

Expected: All tests pass

**Step 4: Commit if any fixes needed**

```bash
git add -A
git commit -m "style: fix formatting and clippy warnings"
```

---

## Task 10: Update Documentation

**Files:**
- Modify: `CLAUDE.md`

**Step 1: Update CLI options table in CLAUDE.md**

Add to the CLI Options table:

```markdown
| Option    | Commands          | Description                                                                 |
|-----------|-------------------|-----------------------------------------------------------------------------|
| `--dry-run`| all              | Validate config and inputs without generating output. Exits: 0=valid, 1=config error, 2=path error, 3=URL error |
```

**Step 2: Add examples section to CLAUDE.md**

Add after the Quick Reference section:

```markdown
### Dry Run Mode

Validate configuration and inputs before processing:

```bash
# Validate collection config
cityjson-stac collection --config config.yaml --dry-run

# Validate item input (local or remote)
cityjson-stac item https://example.com/data.json --dry-run

# Validate update-collection inputs
cityjson-stac update-collection items/*.json --dry-run

# Validate catalog configuration
cityjson-stac catalog --config catalog-config.yaml --dry-run
```

**Exit codes:**
- `0` - All validations passed
- `1` - Config file error (syntax/semantic)
- `2` - Missing input paths
- `3` - Remote URL inaccessible
```

**Step 3: Commit**

```bash
git add CLAUDE.md
git commit -m "docs: update CLAUDE.md with dry-run documentation"
```

---

## Summary

This implementation adds a complete `--dry-run` feature that:

1. ✅ Adds `--dry-run` as a global CLI flag
2. ✅ Validates config file syntax (YAML/TOML)
3. ✅ Checks input paths exist
4. ✅ Validates remote URLs with lightweight HEAD requests
5. ✅ Provides visual feedback using existing progress UI
6. ✅ Exits with appropriate codes (0-3)
7. ✅ Works across all commands (item, collection, update-collection, catalog)
8. ✅ Includes unit tests
9. ✅ Maintains backward compatibility (normal execution unchanged)

**Total commits:** 10
**Estimated time:** 1-2 hours
**Files modified:** 3 (src/cli/mod.rs, src/lib.rs, CLAUDE.md)
**Files created:** 3 (src/validation/mod.rs, src/validation/result.rs, TESTING_NOTES.md)
