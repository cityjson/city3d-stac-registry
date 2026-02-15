# City 3D STAC

A command-line tool for generating STAC (SpatioTemporal Asset Catalog) metadata from CityJSON datasets.

## Overview

STAC is the widely-adopted metadata standard for geospatial data, but it lacks native support for 3D city models. This tool bridges that gap by automatically generating STAC Items and Collections from various CityJSON-format files by traversing directories and extracting comprehensive metadata.

### Supported Formats

- **CityJSON** (`.json`) - Standard CityJSON files
- **CityJSONTextSequences** (`.jsonl`) - Line-delimited CityJSON features
- **FlatCityBuf** (`.fcb`) - Binary columnar format for CityJSON
- **CityParquet** (`.parquet`) - Planned future support

## Features

- Generate STAC Items from individual files
- Generate STAC Collections by traversing directories
- Custom STAC extension for 3D city model metadata
- Support for multiple CityJSON formats
- Extract rich metadata including:
  - 3D bounding boxes
  - Coordinate reference systems
  - Levels of Detail (LOD)
  - City object types and counts
  - Semantic attribute schemas
  - Coordinate transforms

## Installation

### From source

```bash
git clone https://github.com/HideBa/city3d-stac
cd city3d-stac
cargo build --release
```

The binary will be available at `./target/release/c ity3dstac`.

## Quick Start

### Generate STAC Item from a single file

```bash
city3dstac item building.json -o building_stac.json
```

With custom metadata:

```bash
city3dstac item building.json \
  --title "City Hall Building Model" \
  --description "LOD2 model with semantic attributes" \
  -o building_item.json
```

### Generate STAC Collection from a directory

```bash
city3dstac collection ./data/ \
  --title "Rotterdam 3D City Model" \
  --description "Buildings, terrain, and infrastructure in LOD2" \
  --license "CC-BY-4.0" \
  -o ./stac_catalog
```

The command will:

- Scan the directory for supported files (`.json`, `.jsonl`, `.fcb`)
- Generate a STAC Item for each file
- Aggregate metadata into a STAC Collection
- Create the output structure:
  ```text
  stac_catalog/
  ├── collection.json
  └── items/
      ├── building1_item.json
      ├── building2_item.json
      └── ...
  ```

## CLI Reference

### Global Options

| Option          | Description                   |
| --------------- | ----------------------------- |
| `-v, --verbose` | Enable verbose logging output |
| `-h, --help`    | Print help information        |
| `-V, --version` | Print version information     |

### Commands

#### `item` - Generate STAC Item from single file

Generate a STAC Item from a single CityJSON file.

```bash
city3dstac item <FILE> [OPTIONS]
```

**Arguments:**

| Argument | Description     |
| -------- | --------------- |
| `<FILE>` | Input file path |

**Options:**

| Option                     | Description                                         |
| -------------------------- | --------------------------------------------------- |
| `-o, --output <PATH>`      | Output file path (default: `<input>.item.json`)     |
| `--id <ID>`                | Custom STAC Item ID (default: file stem)            |
| `--title <TITLE>`          | Item title                                          |
| `-d, --description <DESC>` | Item description                                    |
| `-c, --collection <ID>`    | Parent collection ID (adds collection link)         |
| `--base-url <URL>`         | Base URL for asset hrefs (makes them absolute URLs) |
| `--pretty`                 | Pretty-print JSON output (default: true)            |

**Examples:**

```bash
# Basic usage (output: building.item.json)
city3dstac item building.json

# With custom output path
city3dstac item building.json -o output/building_stac.json

# With metadata
city3dstac item building.json \
  --title "City Hall Building Model" \
  --description "LOD2 model with semantic attributes" \
  -o building_item.json

# With absolute URL for assets (useful for Object Storage)
city3dstac item building.json \
  --base-url https://data.example.com/cityjson/ \
  -o building_item.json
```

---

#### `collection` - Generate STAC Collection from directory

Scan a directory for CityJSON files and generate a STAC Collection with Items.

```bash
city3dstac collection <DIRECTORY> [OPTIONS]
```

**Arguments:**

| Argument      | Description       |
| ------------- | ----------------- |
| `<DIRECTORY>` | Directory to scan |

**Options:**

| Option                     | Description                                         |
| -------------------------- | --------------------------------------------------- |
| `-o, --output <PATH>`      | Output directory (default: `./stac_output`)         |
| `-C, --config <FILE>`      | Configuration file (YAML or TOML)                   |
| `--id <ID>`                | Collection ID (default: directory name)             |
| `--title <TITLE>`          | Collection title                                    |
| `-d, --description <DESC>` | Collection description                              |
| `-l, --license <LICENSE>`  | Data license (default: `proprietary`)               |
| `-r, --recursive`          | Scan subdirectories recursively (default: true)     |
| `--max-depth <N>`          | Maximum directory depth                             |
| `--skip-errors`            | Skip files with errors (default: true)              |
| `--base-url <URL>`         | Base URL for asset hrefs (makes them absolute URLs) |
| `--pretty`                 | Pretty-print JSON output (default: true)            |

**Output Structure:**

```
stac_output/
├── collection.json
└── items/
    ├── building1_item.json
    ├── building2_item.json
    └── ...
```

**Examples:**

```bash
# Basic usage
city3dstac collection ./data/

# With metadata and custom output
city3dstac collection ./data/ \
  --title "Rotterdam 3D City Model" \
  --description "Buildings, terrain, and infrastructure in LOD2" \
  --license "CC-BY-4.0" \
  -o ./stac_catalog

# With absolute URLs for assets
city3dstac collection ./data/ \
  --base-url https://data.example.com/cityjson/ \
  -o ./stac_catalog

# Non-recursive with depth limit
city3dstac collection ./data/ --max-depth 2

# With verbose logging
city3dstac collection ./data/ -v
```

---

#### `catalog` - Generate STAC Catalog from multiple directories

Generate a STAC Catalog by aggregating multiple directories (collections).

```bash
city3dstac catalog <DIRS>... [OPTIONS]
```

**Arguments:**

| Argument    | Description                                 |
| ----------- | ------------------------------------------- |
| `<DIRS>...` | Input directories to include as collections |

**Options:**

| Option                     | Description                                          |
| -------------------------- | ---------------------------------------------------- |
| `-o, --output <PATH>`      | Output directory (default: `./catalog`)              |
| `-C, --config <FILE>`      | Configuration file (YAML or TOML)                    |
| `--id <ID>`                | Catalog ID (default: output dir name)                |
| `--title <TITLE>`          | Catalog title                                        |
| `-d, --description <DESC>` | Catalog description                                  |
| `-l, --license <LICENSE>`  | License for sub-collections (default: `proprietary`) |
| `--base-url <URL>`         | Base URL for catalog child links                     |
| `--pretty`                 | Pretty-print JSON output (default: true)             |

**Examples:**

```bash
# Basic usage
city3dstac catalog ./data1 ./data2 -o ./my-catalog

# With metadata
city3dstac catalog ./data1 ./data2 \
  --title "My City Models" \
  --description "A catalog of city models" \
  -o ./my-catalog

# Using a configuration file (YAML or TOML)
city3dstac catalog --config catalog.toml -o ./my-catalog
```

---

#### `update-collection` / `aggregate` - Generate STAC Collection from existing items

Generate a STAC Collection by aggregating metadata from existing STAC Item files. This is useful for **Object Storage** workflows where STAC Items are generated individually and then need to be combined into a collection.

```bash
city3dstac update-collection <ITEMS>... [OPTIONS]
city3dstac aggregate <ITEMS>... [OPTIONS]  # alias
```

**Arguments:**

| Argument     | Description                       |
| ------------ | --------------------------------- |
| `<ITEMS>...` | STAC Item JSON files to aggregate |

**Options:**

| Option                     | Description                                        |
| -------------------------- | -------------------------------------------------- |
| `-o, --output <PATH>`      | Output file path (default: `./collection.json`)    |
| `--id <ID>`                | Collection ID (default: output file stem)          |
| `--title <TITLE>`          | Collection title                                   |
| `-d, --description <DESC>` | Collection description                             |
| `-l, --license <LICENSE>`  | Data license (default: `proprietary`)              |
| `--items-base-url <URL>`   | Base URL for item links (makes them absolute URLs) |
| `--skip-errors`            | Skip items with parsing errors (default: true)     |
| `--pretty`                 | Pretty-print JSON output (default: true)           |

**Aggregated Metadata:**

The command aggregates the following 3D City Models extension properties from all items:

| Property                   | Aggregation Method                      |
| -------------------------- | --------------------------------------- |
| `city3d:encoding`          | Unique list of all encodings            |
| `city3d:version`           | Unique list of all versions             |
| `city3d:lods`              | Merged, sorted list of all LODs         |
| `city3d:co_types`          | Merged, sorted list of all object types |
| `city3d:city_objects`      | Statistics: min, max, total             |
| `city3d:semantic_surfaces` | True if any item has them               |
| `city3d:textures`          | True if any item has them               |
| `city3d:materials`         | True if any item has them               |
| `proj:epsg`                | Unique list of all EPSG codes           |
| `bbox` (spatial)           | Merged bounding box of all items        |

**Examples:**

```bash
# Aggregate items with relative links
city3dstac update-collection item1.json item2.json item3.json -o collection.json

# Using glob pattern
city3dstac update-collection items/*.json -o collection.json

# Using the alias
city3dstac aggregate items/*.json -o collection.json

# With collection metadata
city3dstac update-collection items/*.json \
  --id "rotterdam-3d" \
  --title "Rotterdam 3D City Model" \
  --description "LOD2 buildings from Rotterdam" \
  --license "CC-BY-4.0" \
  -o collection.json

# With absolute URLs for item links (useful for Object Storage)
city3dstac update-collection items/*.json \
  --items-base-url https://example.com/stac/items/ \
  -o collection.json
```

**Object Storage Workflow Example:**

```bash
# Step 1: Generate STAC Items individually (can be parallelized)
city3dstac item building1.json --base-url https://storage.example.com/data/ -o items/building1.json
city3dstac item building2.json --base-url https://storage.example.com/data/ -o items/building2.json
city3dstac item building3.json --base-url https://storage.example.com/data/ -o items/building3.json

# Step 2: Aggregate all items into a collection
city3dstac update-collection items/*.json \
  --items-base-url https://storage.example.com/stac/items/ \
  --title "City Buildings Collection" \
  -o collection.json
```

---

### Filename Collision Handling

When processing a collection with files that have the same stem but different extensions (e.g., `delft.city.json` and `delft.city.jsonl`), item IDs and filenames get format-specific suffixes:

| Format                 | Suffix   | Example            |
| ---------------------- | -------- | ------------------ |
| CityJSON (`.json`)     | `_cj`    | `delft.city_cj`    |
| CityJSONSeq (`.jsonl`) | `_cjseq` | `delft.city_cjseq` |
| FlatCityBuf (`.fcb`)   | `_fcb`   | `delft.city_fcb`   |

## Documentation

- [AGENTS.md](AGENTS.md) / [CLAUDE.md](CLAUDE.md) - Project overview and coding guidelines for AI agents
- [DESIGN_DOC.md](DESIGN_DOC.md) - Detailed technical architecture and implementation
- [STAC_EXTENSION.md](STAC_EXTENSION.md) - CityJSON STAC extension specification

## Project Status

✅ **Core Implementation Complete** - The tool is functional with:

- CityJSON format support (`.json` files)
- CityJSON Sequences support (`.jsonl` files)
- FlatCityBuf support (`.fcb` files)
- STAC Item and Collection generation
- Full CLI with `item`, `collection`, `catalog`, and `update-collection` commands
- TOML and YAML configuration support
- 135+ unit and integration tests passing
- Custom CityJSON STAC extension

## Contributing

Contributions are welcome! Please see the design documents for implementation details and architectural decisions.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## References

- [STAC Specification](https://stacspec.org/)
- [CityJSON](https://www.cityjson.org/)
- [CityJSON Sequences](https://github.com/cityjson/cjseq)
- [FlatCityBuf](https://github.com/cityjson/flatcitybuf)
