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

```bash
cargo install cityjson-stac
```

Or build from source:

```bash
git clone https://github.com/cityjson/cityjson-stac.git
cd cityjson-stac
cargo build --release
```

## Quick Start

Generate a STAC Item from a single file:

```bash
cityjson-stac item building.json -o building_stac.json
```

Generate a STAC Collection from a directory:

```bash
cityjson-stac collection ./data/ \
  --title "City Buildings Dataset" \
  --description "Building models in LOD2" \
  -o ./stac_catalog
```

## Documentation

- [Design Document](DESIGN.md) - Architecture and implementation design
- [STAC Extension Specification](STAC_EXTENSION.md) - Custom extension for CityJSON
- [API Design](API_DESIGN.md) - Trait definitions and code structure

## Project Status

🚧 **Under Development** - This project is in the design and initial implementation phase.

## Contributing

Contributions are welcome! Please see the design documents for implementation details and architectural decisions.

## License

[To be determined]

## References

- [STAC Specification](https://stacspec.org/)
- [CityJSON](https://www.cityjson.org/)
- [CityJSON Sequences](https://github.com/cityjson/cjseq)
- [FlatCityBuf](https://github.com/cityjson/flatcitybuf)