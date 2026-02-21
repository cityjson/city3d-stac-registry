# STAC 3D City Models Extension

**Extension Name:** 3D City Models
**Extension Prefix:** `city3d`
**Extension Version:** 0.1.0
**Extension URL:** https://cityjson.github.io/stac-city3d/v0.1.0/schema.json
**Scope:** Item, Collection

## Purpose

This extension defines properties for STAC Items and Collections describing 3D city model datasets. It supports multiple encoding formats including CityJSON, CityGML, and other 3D geospatial formats.

This tool implements the official [STAC 3D City Models Extension](https://stac-extensions.github.io/3d-city-models/) specification.

---

## Item-Level Properties

Properties in the STAC Item `properties` object:

| Field Name                 | Type          | Required | Description                                |
| -------------------------- | ------------- | -------- | ------------------------------------------ |
| `city3d:encoding`          | string        | Yes\*    | Encoding format (see list below)           |
| `city3d:version`           | string        | No       | Specification version (e.g., "2.0", "3.0") |
| `city3d:encoding_version`  | string        | No       | Encoding-specific version                  |
| `city3d:city_objects`      | integer       | No       | Number of city objects                     |
| `city3d:lods`              | array[string] | No       | Available levels of detail                 |
| `city3d:co_types`          | array[string] | No       | City object types present                  |
| `city3d:attributes`        | array[object] | No       | Attribute schema definitions               |
| `city3d:semantic_surfaces` | boolean       | No       | Has semantic surface information           |
| `city3d:textures`          | boolean       | No       | Has texture information                    |
| `city3d:materials`         | boolean       | No       | Has material information                   |

_At least one `city3d:_` field is required.

---

## Collection-Level Properties

Properties in the STAC Collection `summaries` object:

| Field Name                 | Type          | Description                             |
| -------------------------- | ------------- | --------------------------------------- |
| `city3d:encoding`          | array/string  | All encoding formats in collection      |
| `city3d:version`           | array[string] | All versions in collection              |
| `city3d:encoding_version`  | array[string] | All encoding versions                   |
| `city3d:lods`              | array[string] | All LODs across collection              |
| `city3d:co_types`          | array[string] | All city object types across collection |
| `city3d:city_objects`      | object        | Statistics: `{min, max, total}`         |
| `city3d:semantic_surfaces` | boolean       | True if any item has them               |
| `city3d:textures`          | boolean       | True if any item has them               |
| `city3d:materials`         | boolean       | True if any item has them               |

---

## Property Details

### `city3d:encoding`

3D city model encoding format. Can be a single string or array of strings for multi-format datasets.

**Common values:**

**JSON-based:**

- `"CityJSON"` - Standard CityJSON (.json)
- `"CityJSONSeq"` - CityJSON Text Sequences (.jsonl)
- `"FlatCityBuf"` - FlatBuffers-based format (.fcb)
- `"CityParquet"` - Parquet-based format
- `"CityGML"` - OGC CityGML format
- `"KML/COLLADA"` - KML with COLLADA
- `"3Tiles"` - 3D Tiles format
- `"I3S"` - Esri I3S format
- `"OBJ"` - Wavefront OBJ
- `"GLTF"` - glTF format
- `"GLB"` - Binary glTF

### `city3d:version`

Specification version of the format. Pattern: semantic version (e.g., "2.0", "1.1", "3.0").

### `city3d:encoding_version`

Version of the specific encoding format when it has its own versioning independent of the base specification. For example, FlatCityBuf may have version "0.2.0" regardless of CityJSON version.

### `city3d:city_objects`

**Item:** Integer count of city objects.

**Collection:** Statistics object:

```json
{
  "min": 45,
  "max": 5000,
  "total": 125432
}
```

### `city3d:lods`

Levels of Detail available. Supports decimal values per Biljecki et al. specification (e.g., "2.2").

```json
"city3d:lods": ["1", "2", "2.2", "3"]
```

### `city3d:co_types`

City object types present. Includes both standard types and extension types (prefixed with "+").

**Standard Types:** Building, BuildingPart, BuildingInstallation, BuildingConstructiveElement, BuildingFurniture, BuildingStorey, BuildingRoom, BuildingUnit, Bridge, BridgePart, BridgeInstallation, BridgeConstructiveElement, BridgeRoom, BridgeFurniture, CityFurniture, CityObjectGroup, GenericCityObject, LandUse, OtherConstruction, PlantCover, SolitaryVegetationObject, Railway, Road, TINRelief, TransportSquare, Tunnel, TunnelPart, TunnelInstallation, TunnelConstructiveElement, TunnelHollowSpace, TunnelFurniture, WaterBody, WaterSurface, WaterGroundSurface, Waterway

```json
"city3d:co_types": ["Building", "BuildingPart", "TINRelief"]
```

### `city3d:attributes`

Attribute schema definitions for semantic attributes on city objects.

| Property    | Type    | Required | Description                                  |
| ----------- | ------- | -------- | -------------------------------------------- |
| name        | string  | Yes      | Attribute name                               |
| type        | string  | Yes      | STRING, NUMBER, BOOLEAN, DATE, ARRAY, OBJECT |
| description | string  | No       | Human-readable description                   |
| required    | boolean | No       | Whether always present                       |

```json
"city3d:attributes": [
  { "name": "yearOfConstruction", "type": "NUMBER" },
  { "name": "function", "type": "STRING", "description": "Building usage" }
]
```

### `city3d:semantic_surfaces`

Boolean indicating whether the dataset contains semantic surfaces (e.g., roofs, walls, ground surfaces) that provide detailed geometry breakdown with specific semantic meaning.

### `city3d:textures`

Boolean indicating whether the dataset includes texture information for visual appearance of surfaces.

### `city3d:materials`

Boolean indicating whether the dataset includes material information (e.g., color, shininess, transparency) for surface appearance.

---

## Referenced Extensions

This extension incorporates properties from these STAC extensions via `$ref`:

- **[Projection v2.0.0](https://stac-extensions.github.io/projection/v2.0.0/schema.json)**: `proj:epsg`, `proj:wkt2`, `proj:projjson`, `proj:centroid`, `proj:geometry`
- **[File v2.1.0](https://stac-extensions.github.io/file/v2.1.0/schema.json)**: `file:size`, `file:checksum`, `file:values`
- **[Stats v0.2.0](https://stac-extensions.github.io/stats/v0.2.0/schema.json)**: `stats:...`

---

## Examples

### STAC Item (CityJSON)

```json
{
  "stac_version": "1.1.0",
  "stac_extensions": [
    "https://cityjson.github.io/stac-city3d/v0.1.0/schema.json",
    "https://stac-extensions.github.io/projection/v2.0.0/schema.json"
  ],
  "type": "Feature",
  "id": "rotterdam_buildings_lod2",
  "bbox": [4.46, 51.91, 4.49, 51.93],
  "geometry": {
    "type": "Polygon",
    "coordinates": [
      [
        [4.46, 51.91],
        [4.49, 51.91],
        [4.49, 51.93],
        [4.46, 51.93],
        [4.46, 51.91]
      ]
    ]
  },
  "properties": {
    "datetime": "2023-05-15T00:00:00Z",
    "proj:epsg": 7415,
    "city3d:encoding": "CityJSON",
    "city3d:version": "2.0",
    "city3d:city_objects": 1523,
    "city3d:lods": ["2", "2.2"],
    "city3d:co_types": ["Building", "BuildingPart", "BuildingInstallation"],
    "city3d:semantic_surfaces": true
  },
  "assets": {
    "data": {
      "href": "./rotterdam_buildings_lod2.city.json",
      "type": "application/json",
      "roles": ["data"]
    }
  },
  "links": [
    { "rel": "self", "href": "./rotterdam_buildings_lod2_item.json" },
    { "rel": "parent", "href": "./collection.json" }
  ]
}
```

### STAC Item (CityGML)

```json
{
  "stac_version": "1.1.0",
  "stac_extensions": [
    "https://cityjson.github.io/stac-city3d/v0.1.0/schema.json",
    "https://stac-extensions.github.io/projection/v2.1.0/schema.json"
  ],
  "type": "Feature",
  "id": "hamburg_citygml_lod2",
  "bbox": [9.93, 53.55, 9.95, 53.56],
  "geometry": {
    "type": "Polygon",
    "coordinates": [
      [
        [9.93, 53.55],
        [9.95, 53.55],
        [9.95, 53.56],
        [9.93, 53.56],
        [9.93, 53.55]
      ]
    ]
  },
  "properties": {
    "datetime": "2023-05-15T00:00:00Z",
    "proj:epsg": 25832,
    "city3d:encoding": "CityGML",
    "city3d:version": "2.0",
    "city3d:city_objects": 42800,
    "city3d:lods": ["0", "1", "2"],
    "city3d:co_types": ["Building", "BuildingPart", "Road", "WaterBody"],
    "city3d:semantic_surfaces": true,
    "city3d:textures": true,
    "city3d:materials": true
  },
  "assets": {
    "data": {
      "href": "./hamburg_lod2.gml",
      "type": "application/gml+xml",
      "roles": ["data"]
    }
  }
}
```

### STAC Collection

```json
{
  "stac_version": "1.1.0",
  "stac_extensions": [
    "https://cityjson.github.io/stac-city3d/v0.1.0/schema.json",
    "https://stac-extensions.github.io/projection/v2.0.0/schema.json"
  ],
  "type": "Collection",
  "id": "rotterdam_3dcity_2023",
  "title": "Rotterdam 3D City Model 2023",
  "description": "3D city model with buildings and terrain in multiple LODs",
  "license": "CC-BY-4.0",
  "extent": {
    "spatial": { "bbox": [[4.42, 51.88, 4.6, 51.98]] },
    "temporal": { "interval": [["2023-05-15T00:00:00Z", null]] }
  },
  "summaries": {
    "proj:epsg": [7415],
    "city3d:encoding": ["CityJSON", "FlatCityBuf"],
    "city3d:version": ["2.0"],
    "city3d:city_objects": { "min": 45, "max": 5000, "total": 125432 },
    "city3d:lods": ["0", "1", "2", "2.2", "3"],
    "city3d:co_types": ["Building", "BuildingPart", "TINRelief", "WaterBody"],
    "city3d:semantic_surfaces": true,
    "city3d:textures": false
  },
  "links": [
    { "rel": "self", "href": "./collection.json" },
    { "rel": "item", "href": "./items/buildings_item.json" }
  ]
}
```

---

## Best Practices

1. **3D Bounding Box**: Use 6-element bbox `[xmin, ymin, zmin, xmax, ymax, zmax]` when z-values are meaningful
2. **Geometry**: Provide 2D footprint polygon in WGS84 for map display
3. **Temporal**: Use `datetime` for single timestamp, or `start/end_datetime` for ranges
4. **Assets**: Include primary data file with `"roles": ["data"]`
5. **Semantic Surfaces**: Set `city3d:semantic_surfaces: true` when surfaces are explicitly defined
6. **Textures/Materials**: Only set these flags when appearance information is actually present

---

## Schema Validation

The JSON Schema for this extension is:

```
https://cityjson.github.io/stac-city3d/v0.1.0/schema.json
```

Local development copy available at:

```
stac-cityjson-extension/json-schema/schema.json
```

---

## External References

- [STAC Specification](https://stacspec.org/)
- [3D City Models Extension Repository](https://github.com/stac-extensions/3d-city-models)
- [CityJSON Specification](https://www.cityjson.org/specs/)
- [CityGML Standard](https://www.ogc.org/standards/citygml)
