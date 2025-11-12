# CLI Design and Implementation

## Command Structure

### Overview

```
cityjson-stac <COMMAND> [OPTIONS]

COMMANDS:
    item        Generate STAC Item from a single file
    collection  Generate STAC Collection from directory
    validate    Validate STAC file against CityJSON extension
    help        Print help information
```

## Commands in Detail

### 1. `item` Command

Generate a STAC Item from a single CityJSON-format file.

#### Signature

```bash
cityjson-stac item <FILE> [OPTIONS]
```

#### Arguments

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `FILE` | Path | Yes | Path to input file (.json, .jsonl, .fcb) |

#### Options

| Option | Short | Type | Default | Description |
|--------|-------|------|---------|-------------|
| `--output` | `-o` | Path | `<file>_item.json` | Output file path |
| `--id` | | String | Filename | Custom STAC Item ID |
| `--title` | | String | Filename | Item title |
| `--description` | `-d` | String | None | Item description |
| `--datetime` | | ISO8601 | Now | Dataset timestamp |
| `--collection` | `-c` | String | None | Parent collection ID |
| `--license` | `-l` | String | "proprietary" | Data license |
| `--pretty` | | Flag | true | Pretty-print JSON output |
| `--verbose` | `-v` | Flag | false | Verbose output |

#### Examples

```bash
# Basic usage
cityjson-stac item building.json

# Custom output path
cityjson-stac item building.json -o stac/building_metadata.json

# With metadata
cityjson-stac item building.json \
  --title "City Hall Building Model" \
  --description "LOD2 model with semantic attributes" \
  --datetime "2023-05-15T00:00:00Z" \
  --license "CC-BY-4.0"

# Assign to collection
cityjson-stac item building.json \
  --collection "rotterdam-buildings-2023" \
  -o stac/items/building_001.json
```

#### Output

Creates a STAC Item JSON file with:
- Standard STAC Item structure
- CityJSON extension properties
- Asset pointing to source file
- Links (self, parent, collection)

#### Exit Codes

- `0`: Success
- `1`: File not found
- `2`: Unsupported format
- `3`: Metadata extraction error
- `4`: Output write error

### 2. `collection` Command

Generate a STAC Collection from a directory of CityJSON files.

#### Signature

```bash
cityjson-stac collection <DIRECTORY> [OPTIONS]
```

#### Arguments

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `DIRECTORY` | Path | Yes | Directory to scan for files |

#### Options

| Option | Short | Type | Default | Description |
|--------|-------|------|---------|-------------|
| `--output` | `-o` | Path | `./stac_output` | Output directory |
| `--id` | | String | Dir name | Collection ID |
| `--title` | | String | Dir name | Collection title |
| `--description` | `-d` | String | None | Collection description |
| `--license` | `-l` | String | "proprietary" | Data license |
| `--recursive` | `-r` | Flag | true | Scan subdirectories |
| `--max-depth` | | Number | None | Maximum directory depth |
| `--extensions` | `-e` | String[] | All supported | File extensions to include |
| `--parallel` | `-p` | Flag | false | Parallel processing |
| `--skip-errors` | | Flag | true | Continue on file errors |
| `--pretty` | | Flag | true | Pretty-print JSON |
| `--verbose` | `-v` | Flag | false | Verbose output |

#### Examples

```bash
# Basic collection generation
cityjson-stac collection ./buildings/

# Custom metadata
cityjson-stac collection ./data/ \
  --title "Rotterdam 3D City Model" \
  --description "Buildings, terrain, and infrastructure in LOD2" \
  --license "CC-BY-4.0" \
  -o ./stac_catalog

# Recursive with depth limit
cityjson-stac collection ./city_data/ \
  --recursive \
  --max-depth 3 \
  -o ./stac

# Only specific formats
cityjson-stac collection ./mixed_data/ \
  --extensions json jsonl \
  -o ./stac

# Parallel processing for large datasets
cityjson-stac collection ./large_dataset/ \
  --parallel \
  --verbose \
  -o ./stac
```

#### Output Structure

Creates directory structure:

```
<output_dir>/
├── collection.json          # STAC Collection
├── catalog.json            # Optional root catalog
└── items/
    ├── item_001.json       # STAC Item for file 1
    ├── item_002.json       # STAC Item for file 2
    └── ...
```

#### Progress Output

When `--verbose` is enabled:

```
Scanning directory: ./buildings/
Found 156 files (123 .json, 33 .fcb)
Processing files: [========================================] 156/156
  ✓ building_001.json (1523 objects, LOD2)
  ✓ building_002.json (892 objects, LOD1-2)
  ⚠ terrain_001.fcb (skipped: parse error)
  ...
Generated 155 items
Collection bounds: [4.46, 51.91, -5.0, 4.49, 51.93, 100.0]
Collection saved: ./stac_output/collection.json
Items saved: ./stac_output/items/
```

#### Exit Codes

- `0`: Success (all files processed)
- `1`: Directory not found
- `2`: No supported files found
- `3`: Partial success (some files failed, but --skip-errors enabled)
- `4`: Output write error

### 3. `validate` Command

Validate STAC JSON against CityJSON extension schema.

#### Signature

```bash
cityjson-stac validate <STAC_FILE> [OPTIONS]
```

#### Arguments

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `STAC_FILE` | Path | Yes | STAC Item or Collection file |

#### Options

| Option | Short | Type | Description |
|--------|-------|------|-------------|
| `--schema` | `-s` | Path | Custom schema file path |
| `--strict` | | Flag | Fail on warnings |
| `--verbose` | `-v` | Flag | Show detailed validation info |

#### Examples

```bash
# Validate STAC Item
cityjson-stac validate building_item.json

# Validate with custom schema
cityjson-stac validate collection.json --schema ./custom_schema.json

# Strict mode
cityjson-stac validate item.json --strict
```

#### Output

```
Validating: building_item.json
✓ Valid STAC Item (version 1.0.0)
✓ CityJSON extension properties present
✓ All required fields present
✓ Property types valid
⚠ Warning: cj:attributes field is empty
Result: VALID (1 warning)
```

#### Exit Codes

- `0`: Valid
- `1`: Invalid STAC structure
- `2`: Extension validation failed
- `3`: File not found

## Global Options

Available for all commands:

| Option | Short | Description |
|--------|-------|-------------|
| `--help` | `-h` | Show help information |
| `--version` | `-V` | Show version |
| `--quiet` | `-q` | Suppress output except errors |

## Implementation Details

### Argument Parsing with `clap`

```rust
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "cityjson-stac")]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Suppress output except errors
    #[arg(short, long, global = true)]
    quiet: bool,

    /// Verbose output
    #[arg(short, long, global = true)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate STAC Item from a single file
    Item {
        /// Input file path
        file: PathBuf,

        /// Output file path
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// STAC Item ID
        #[arg(long)]
        id: Option<String>,

        /// Item title
        #[arg(long)]
        title: Option<String>,

        /// Item description
        #[arg(short, long)]
        description: Option<String>,

        /// Dataset timestamp (ISO8601)
        #[arg(long)]
        datetime: Option<String>,

        /// Parent collection ID
        #[arg(short, long)]
        collection: Option<String>,

        /// Data license
        #[arg(short, long, default_value = "proprietary")]
        license: String,

        /// Pretty-print JSON
        #[arg(long, default_value_t = true)]
        pretty: bool,
    },

    /// Generate STAC Collection from directory
    Collection {
        /// Directory to scan
        directory: PathBuf,

        /// Output directory
        #[arg(short, long, default_value = "./stac_output")]
        output: PathBuf,

        /// Collection ID
        #[arg(long)]
        id: Option<String>,

        /// Collection title
        #[arg(long)]
        title: Option<String>,

        /// Collection description
        #[arg(short, long)]
        description: Option<String>,

        /// Data license
        #[arg(short, long, default_value = "proprietary")]
        license: String,

        /// Scan subdirectories recursively
        #[arg(short, long, default_value_t = true)]
        recursive: bool,

        /// Maximum directory depth
        #[arg(long)]
        max_depth: Option<usize>,

        /// File extensions to include
        #[arg(short, long, value_delimiter = ',')]
        extensions: Option<Vec<String>>,

        /// Enable parallel processing
        #[arg(short, long)]
        parallel: bool,

        /// Skip files with errors
        #[arg(long, default_value_t = true)]
        skip_errors: bool,

        /// Pretty-print JSON
        #[arg(long, default_value_t = true)]
        pretty: bool,
    },

    /// Validate STAC file
    Validate {
        /// STAC file to validate
        stac_file: PathBuf,

        /// Custom schema file
        #[arg(short, long)]
        schema: Option<PathBuf>,

        /// Strict mode (fail on warnings)
        #[arg(long)]
        strict: bool,
    },
}
```

### Command Handlers

```rust
// src/cli/mod.rs

pub fn run() -> Result<()> {
    let cli = Cli::parse();

    // Set up logging based on verbosity
    setup_logging(cli.quiet, cli.verbose)?;

    match cli.command {
        Commands::Item { file, output, .. } => {
            handle_item_command(file, output, ...)?;
        }
        Commands::Collection { directory, output, .. } => {
            handle_collection_command(directory, output, ...)?;
        }
        Commands::Validate { stac_file, .. } => {
            handle_validate_command(stac_file, ...)?;
        }
    }

    Ok(())
}

fn handle_item_command(
    file: PathBuf,
    output: Option<PathBuf>,
    // ... other params
) -> Result<()> {
    // 1. Validate file exists
    if !file.exists() {
        return Err(CityJsonStacError::IoError(
            std::io::Error::new(ErrorKind::NotFound, "File not found")
        ));
    }

    // 2. Get reader
    let reader = get_reader(&file)?;

    // 3. Build STAC Item
    let item = StacItemBuilder::new(id.unwrap_or_else(|| generate_id(&file)))
        .bbox(reader.bbox()?)
        .title(title.unwrap_or_else(|| file.file_stem().unwrap().to_string_lossy().to_string()))
        .description(description)
        .cityjson_metadata(reader.as_ref())?
        .data_asset(file.to_string_lossy().to_string(), get_media_type(&file))
        .build()?;

    // 4. Serialize
    let output_path = output.unwrap_or_else(|| generate_output_path(&file));
    let json = if pretty {
        serde_json::to_string_pretty(&item)?
    } else {
        serde_json::to_string(&item)?
    };

    // 5. Write output
    std::fs::write(&output_path, json)?;

    if !quiet {
        println!("✓ Generated STAC Item: {}", output_path.display());
    }

    Ok(())
}
```

### Progress Reporting

```rust
use indicatif::{ProgressBar, ProgressStyle};

fn handle_collection_command(
    directory: PathBuf,
    output: PathBuf,
    // ... params
) -> Result<()> {
    // 1. Find all files
    let files = find_files(&directory, recursive, max_depth, extensions)?;

    if files.is_empty() {
        return Err(CityJsonStacError::NoFilesFound);
    }

    // 2. Set up progress bar
    let pb = if verbose {
        Some(ProgressBar::new(files.len() as u64))
    } else {
        None
    };

    if let Some(pb) = &pb {
        pb.set_style(
            ProgressStyle::default_bar()
                .template("[{bar:40}] {pos}/{len} {msg}")?
                .progress_chars("=>-")
        );
    }

    // 3. Process files
    let mut readers = Vec::new();
    let mut errors = Vec::new();

    for file in &files {
        match get_reader(file) {
            Ok(reader) => {
                if let Some(pb) = &pb {
                    let msg = format!("✓ {} ({} objects)",
                        file.file_name().unwrap().to_string_lossy(),
                        reader.city_object_count().unwrap_or(0)
                    );
                    pb.set_message(msg);
                }
                readers.push(reader);
            }
            Err(e) => {
                if skip_errors {
                    errors.push((file.clone(), e));
                    if let Some(pb) = &pb {
                        pb.set_message(format!("⚠ {} (error)", file.display()));
                    }
                } else {
                    return Err(e);
                }
            }
        }

        if let Some(pb) = &pb {
            pb.inc(1);
        }
    }

    if let Some(pb) = &pb {
        pb.finish_with_message("Processing complete");
    }

    // 4. Generate collection
    // ... (implementation)

    // 5. Report results
    if !quiet {
        println!("\n✓ Generated {} items", readers.len());
        if !errors.is_empty() {
            println!("⚠ {} files skipped due to errors", errors.len());
        }
        println!("Collection: {}/collection.json", output.display());
        println!("Items: {}/items/", output.display());
    }

    Ok(())
}
```

## Error Messages

### User-Friendly Errors

```rust
impl fmt::Display for CityJsonStacError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedFormat(ext) => {
                write!(f, "Unsupported file format: {}\n\
                           Supported formats: .json (CityJSON), .jsonl (CityJSONSeq), .fcb (FlatCityBuf)", ext)
            }
            Self::IoError(e) if e.kind() == ErrorKind::NotFound => {
                write!(f, "File or directory not found\n\
                           Please check the path and try again")
            }
            Self::MetadataError(msg) => {
                write!(f, "Failed to extract metadata: {}\n\
                           The file may be corrupted or invalid", msg)
            }
            // ... other errors
        }
    }
}
```

## Shell Completions

Generate completions for various shells:

```rust
use clap_complete::{generate_to, shells::{Bash, Zsh, Fish}};

// In build.rs or separate command
fn generate_completions() {
    let outdir = std::env::var_os("OUT_DIR").unwrap();
    let mut cmd = Cli::command();

    generate_to(Bash, &mut cmd, "cityjson-stac", &outdir)?;
    generate_to(Zsh, &mut cmd, "cityjson-stac", &outdir)?;
    generate_to(Fish, &mut cmd, "cityjson-stac", &outdir)?;
}
```

Install completions:

```bash
# Bash
cityjson-stac completions bash > /usr/share/bash-completion/completions/cityjson-stac

# Zsh
cityjson-stac completions zsh > /usr/local/share/zsh/site-functions/_cityjson-stac

# Fish
cityjson-stac completions fish > ~/.config/fish/completions/cityjson-stac.fish
```

## Configuration File (Optional Future Enhancement)

Support `.cityjson-stac.toml` configuration:

```toml
[defaults]
license = "CC-BY-4.0"
pretty = true

[output]
item_template = "{id}_item.json"
collection_dir = "stac_catalog"

[collection]
recursive = true
skip_errors = true
parallel = false
```

## Testing CLI

```rust
#[cfg(test)]
mod tests {
    use assert_cmd::Command;
    use predicates::prelude::*;
    use tempfile::TempDir;

    #[test]
    fn test_item_command() {
        let mut cmd = Command::cargo_bin("cityjson-stac").unwrap();
        cmd.arg("item")
            .arg("tests/fixtures/building.json")
            .assert()
            .success()
            .stdout(predicate::str::contains("Generated STAC Item"));
    }

    #[test]
    fn test_invalid_file() {
        let mut cmd = Command::cargo_bin("cityjson-stac").unwrap();
        cmd.arg("item")
            .arg("nonexistent.json")
            .assert()
            .failure()
            .stderr(predicate::str::contains("File not found"));
    }
}
```
