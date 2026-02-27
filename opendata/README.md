# Open Data STAC Configurations

This directory contains STAC configuration files for datasets from the [TU Delft 3D Open Cities Portal](https://3d.bk.tudelft.nl/opendata/opencities/).

## Overview

These configurations reference open 3D city model datasets from around the world that are available in **CityJSON** or **CityGML** formats. The data is curated by the 3D Geoinformation Research Group at TU Delft.

## Files

### Catalog Configuration

- **`catalog-config.yaml`** - Main STAC catalog configuration that aggregates all collection configs

### Collection Configurations (CityJSON)

| Config File | Dataset | Format | LoD | Description |
|-------------|---------|--------|-----|-------------|
| `american-cities-config.yaml` | American Cities (USA) | CityJSON/CityGML | LoD1 | 125M buildings, separated by state |
| `netherlands-3d-bag-config.yaml` | Netherlands 3D BAG | CityJSON | LoD1+LoD2 | All 10M+ buildings in the Netherlands |
| `singapore-config.yaml` | Singapore HDB | CityJSON | LoD1 | Public housing buildings |
| `various-cityjson-config.yaml` | Various Cities | CityJSON | LoD2 | The Hague, Montréal, New York, Rotterdam, Vienna, Zürich |

### Collection Configurations (CityGML)

| Config File | Dataset | LoD | Description |
|-------------|---------|-----|-------------|
| `berlin-config.toml` | Berlin, Germany | LoD2 | With textures |
| `brussels-config.yaml` | Brussels, Belgium | LoD2 | Capital city |
| `dresden-config.yaml` | Dresden, Germany | LoD1/2/3 | Multi-LOD with partial textures |
| `espoo-config.yaml` | Espoo, Finland | LoD1-3 | Multi-LOD with textures, multiple classes |
| `estonia-config.yaml` | Estonia (national) | LoD1+LoD2 | Frequently updated |
| `hamburg-config.yaml` | Hamburg, Germany | LoD1+LoD2 | From cadastre + LiDAR |
| `helsinki-config.yaml` | Helsinki, Finland | LoD2 | With textures, multi-format |
| `ingolstadt-config.yaml` | Ingolstadt, Germany | LoD3 | High-detail, MLS-based |
| `japan-plateau.yaml` | Japan PLATEAU | LoD1+LoD2 | 56 cities, national project |
| `linz-config.yaml` | Linz, Austria | LoD2 | Basic city model |
| `luxembourg-config.yaml` | Luxembourg | LoD1+LoD2.3 | National dataset |
| `lyon-config.yaml` | Lyon, France | LoD2 | Multi-temporal (2009/2012/2015), textured |
| `montreal-config.yaml` | Montréal, Canada | LoD2 | Photogrammetry, textured |
| `namur-config.yaml` | Namur, Belgium | LoD2 | Textured, includes bridges & citadel |
| `new-york-doitt-config.yaml` | NYC (DoITT) | LoD2 | LiDAR-based |
| `new-york-tum-config.yaml` | NYC (TUM) | LoD1 | Multi-class, photogrammetry |
| `north-rhine-westphalia-config.yaml` | NRW, Germany | LoD1+LoD2 | State-wide, many cities |
| `potsdam-config.yaml` | Potsdam, Germany | LoD2 | Basic city model |
| `rotterdam-config.yaml` | Rotterdam, Netherlands | LoD2 | Textured, LiDAR-based |
| `the-hague-config.yaml` | The Hague, Netherlands | LoD2 | With terrain |
| `vantaa-config.yaml` | Vantaa, Finland | LoD1+LoD2 | Textured, multi-LOD |
| `vienna-config.yaml` | Vienna, Austria | LoD2 | Semi-automated generation |

## Usage

### Generate the Full Catalog

```bash
# Generate catalog with all collections
city3dstac catalog --config opendata/catalog-config.toml -o ./opendata/catalog_output
```

### Generate Individual Collections

Before using these configs, you need to:

1. **Download the data** from the TU Delft portal or original sources
2. **Update the `inputs` paths** in each config to point to your downloaded files
3. **Run the collection generation**:

```bash
# Example for Netherlands 3D BAG
city3dstac collection --config opendata/netherlands-3d-bag-config.yaml -o ./opendata/netherlands-3d-bag

# Example for Singapore
city3dstac collection --config opendata/singapore-config.yaml -o ./opendata/singapore
```

## Data Sources

All datasets are sourced from the [TU Delft 3D Open Cities Portal](https://3d.bk.tudelft.nl/opendata/opencities/). Please check the original data sources for:

- **License information** (varies by dataset)
- **Download URLs**
- **Citation requirements**
- **Update frequency**

## Notes

- **Bounding boxes** (`bbox`) are left empty for auto-detection from the actual data
- **Coordinate Reference Systems** (`crs`) are set to common values for each region but may need adjustment
- **Input paths** (`inputs`) need to be updated to point to your local data files
- **Some datasets have multiple versions or time periods** - create additional configs if needed

## License

Each dataset has its own license. Please check the individual config files and original sources for specific licensing terms before use.

## Links

- [TU Delft 3D Geoinformation](https://3d.bk.tudelft.nl/)
- [STAC Specification](https://stacspec.org/)
- [CityJSON Specification](https://www.cityjson.org/specs/)
- [CityGML Specification](https://www.ogc.org/standards/citygml)
