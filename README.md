# cityjson-stac

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
git clone https://github.com/cityjson/cityjson-stac.git
cd cityjson-stac
cargo build --release
```

The binary will be available at `./target/release/cityjson-stac`.

## Quick Start

### Generate STAC Item from a single file

```bash
cityjson-stac item building.json -o building_stac.json
```

With custom metadata:

```bash
cityjson-stac item building.json \
  --title "City Hall Building Model" \
  --description "LOD2 model with semantic attributes" \
  -o building_item.json
```

### Generate STAC Collection from a directory

```bash
cityjson-stac collection ./data/ \
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
  ```
  stac_catalog/
  ├── collection.json
  └── items/
      ├── building1_item.json
      ├── building2_item.json
      └── ...
  ```

## CLI Reference

### Commands

#### `item` - Generate STAC Item from single file

```bash
cityjson-stac item <FILE> [OPTIONS]

Arguments:
  <FILE>  Input file path

Options:
  -o, --output <PATH>          Output file path
  --id <ID>                    Custom STAC Item ID
  --title <TITLE>              Item title
  -d, --description <DESC>     Item description
  -c, --collection <ID>        Parent collection ID
  --pretty                     Pretty-print JSON (default: true)
  -v, --verbose                Verbose output
```

#### `collection` - Generate STAC Collection from directory

```bash
cityjson-stac collection <DIRECTORY> [OPTIONS]

Arguments:
  <DIRECTORY>  Directory to scan

Options:
  -o, --output <PATH>          Output directory (default: ./stac_output)
  --id <ID>                    Collection ID
  --title <TITLE>              Collection title
  -d, --description <DESC>     Collection description
  -l, --license <LICENSE>      Data license (default: proprietary)
  -r, --recursive              Scan subdirectories (default: true)
  --max-depth <N>              Maximum directory depth
  --skip-errors                Skip files with errors (default: true)
  --pretty                     Pretty-print JSON (default: true)
  -v, --verbose                Verbose output
```

## Examples

### Basic workflow

```bash
# Create STAC Item for a single CityJSON file
cityjson-stac item building.json

# Process a directory of CityJSON files
cityjson-stac collection ./city_data/ \
  --title "City 3D Models" \
  --recursive \
  -o ./stac_output

# With verbose logging
cityjson-stac collection ./data/ -v
```

## Documentation

- [Design Document](DESIGN.md) - Architecture and implementation design
- [STAC Extension Specification](STAC_EXTENSION.md) - Custom extension for CityJSON
- [API Design](API_DESIGN.md) - Trait definitions and code structure

## Project Status

✅ **Core Implementation Complete** - The tool is functional with:
- CityJSON format support (`.json` files)
- STAC Item and Collection generation
- Full CLI with `item` and `collection` commands
- 46 unit and integration tests passing
- Custom CityJSON STAC extension

🚧 **In Progress:**
- CityJSON Sequences (`.jsonl`) support
- FlatCityBuf (`.fcb`) support

## Contributing

Contributions are welcome! Please see the design documents for implementation details and architectural decisions.

## License

[To be determined]

## References

- [STAC Specification](https://stacspec.org/)
- [CityJSON](https://www.cityjson.org/)
- [CityJSON Sequences](https://github.com/cityjson/cjseq)
- [FlatCityBuf](https://github.com/cityjson/flatcitybuf)