# CityJSON STAC Extension Specification

## Extension Overview

**Extension Name:** CityJSON
**Extension Prefix:** `cj`
**Extension Version:** 1.0.0
**Scope:** Item, Collection
**Maturity:** Proposal

### Purpose
This extension defines additional properties for STAC Items and Collections that describe 3D city model datasets in CityJSON and related formats.

## Namespace

All extension properties use the `cj:` prefix to avoid conflicts with other STAC extensions.

## Extension Properties

### Item-Level Properties

Properties that appear in STAC Item `properties` object:

| Field Name | Type | Description | Required |
|------------|------|-------------|----------|
| `cj:encoding` | string | Encoding format: `CityJSON`, `CityJSONSeq`, `FlatCityBuf`, or `CityParquet` | Yes |
| `cj:version` | string | CityJSON version (e.g., "2.0", "1.1") | Yes |
| `cj:city_objects` | integer | Number of city objects in the file | Yes |
| `cj:lods` | array[string] | Available levels of detail (e.g., ["0", "1", "2"]) | Yes |
| `cj:co_types` | array[string] | City object types present (e.g., ["Building", "Road"]) | Yes |
| `cj:attributes` | array[object] | Attribute schema definitions | No |
| `cj:transform` | object | Coordinate transformation parameters (if used) | No |
| `cj:metadata` | object | Additional CityJSON metadata | No |

### Collection-Level Properties

Properties that appear in STAC Collection `summaries` object:

| Field Name | Type | Description | Required |
|------------|------|-------------|----------|
| `cj:encoding` | array[string] | All encoding formats in collection | Yes |
| `cj:city_objects` | object | Statistics: `{min, max, total}` | Yes |
| `cj:lods` | array[string] | All LODs available across collection | Yes |
| `cj:co_types` | array[string] | All CO types across collection | Yes |

## Property Definitions

### `cj:encoding`

The file format/encoding used for the 3D city model data.

**Type:** string
**Allowed Values:**
- `"CityJSON"` - Standard CityJSON (.json)
- `"CityJSONSeq"` - CityJSON Text Sequences (.jsonl)
- `"FlatCityBuf"` - FlatBuffers-based columnar format (.fcb)
- `"CityParquet"` - Parquet-based format (.parquet) [future]

**Example:**
```json
"cj:encoding": "CityJSON"
```

### `cj:version`

The CityJSON specification version used by the file.

**Type:** string
**Pattern:** Semantic versioning (e.g., "2.0", "1.1", "1.0")

**Example:**
```json
"cj:version": "2.0"
```

### `cj:city_objects`

Number of city objects contained in the file.

**Type:** integer
**Minimum:** 0

**Item Example:**
```json
"cj:city_objects": 1523
```

**Collection Example (in summaries):**
```json
"cj:city_objects": {
  "min": 45,
  "max": 5000,
  "total": 125432
}
```

### `cj:lods`

Levels of Detail (LOD) available in the dataset. LODs represent different geometric complexity levels from 0 (lowest detail) to 3+ (highest detail).

**Type:** array[string]
**Items:** String representation of LOD levels (supports decimals like "2.2")

**Example:**
```json
"cj:lods": ["1", "2", "2.2", "3"]
```

### `cj:co_types`

City object types present in the dataset. Based on CityJSON specification.

**Type:** array[string]
**Items:** CityJSON object type names

**Common Types:**
- Building
- BuildingPart
- BuildingInstallation
- Road
- Railway
- TransportSquare
- TINRelief
- WaterBody
- PlantCover
- SolitaryVegetationObject
- LandUse
- GenericCityObject
- CityFurniture
- Bridge
- BridgePart
- BridgeConstructionElement
- Tunnel
- TunnelPart

**Example:**
```json
"cj:co_types": ["Building", "BuildingPart", "Road", "TINRelief"]
```

### `cj:attributes`

Schema definition for semantic attributes attached to city objects.

**Type:** array[object]

**Attribute Object Properties:**
| Property | Type | Description | Required |
|----------|------|-------------|----------|
| `name` | string | Attribute name | Yes |
| `type` | string | Data type: `String`, `Number`, `Boolean`, `Date`, `Array`, `Object` | Yes |
| `description` | string | Human-readable description | No |
| `required` | boolean | Whether attribute is always present | No |

**Example:**
```json
"cj:attributes": [
  {
    "name": "yearOfConstruction",
    "type": "Number",
    "description": "Year the building was constructed"
  },
  {
    "name": "function",
    "type": "String",
    "description": "Building function/usage type"
  },
  {
    "name": "roofType",
    "type": "String"
  }
]
```

### `cj:transform`

Coordinate transformation parameters when vertex compression is used. Based on CityJSON transform object.

**Type:** object

**Transform Object Properties:**
| Property | Type | Description | Required |
|----------|------|-------------|----------|
| `scale` | array[number] | Scale factors [x, y, z] | Yes |
| `translate` | array[number] | Translation offsets [x, y, z] | Yes |

**Example:**
```json
"cj:transform": {
  "scale": [0.001, 0.001, 0.001],
  "translate": [4629170.0, 5804690.0, 0.0]
}
```

### `cj:metadata`

Additional metadata from the CityJSON file's top-level metadata object.

**Type:** object (free-form)

**Common Properties:**
- `referenceDate`: Date the dataset represents
- `geographicalExtent`: Original CityJSON extent values
- `dataSource`: Source of the 3D data
- `pointOfContact`: Contact information
- Custom metadata fields

**Example:**
```json
"cj:metadata": {
  "referenceDate": "2023-05-15",
  "dataSource": "LiDAR survey 2023",
  "pointOfContact": {
    "contactName": "City GIS Department",
    "emailAddress": "gis@city.gov"
  }
}
```

## STAC Item Example

Complete example of a STAC Item for a CityJSON file:

```json
{
  "stac_version": "1.0.0",
  "stac_extensions": [
    "https://raw.githubusercontent.com/yourusername/cityjson-stac/main/extension.json",
    "https://stac-extensions.github.io/projection/v1.1.0/schema.json"
  ],
  "type": "Feature",
  "id": "rotterdam_buildings_lod2",
  "bbox": [4.46, 51.91, -5.0, 4.49, 51.93, 100.0],
  "geometry": {
    "type": "Polygon",
    "coordinates": [[
      [4.46, 51.91],
      [4.49, 51.91],
      [4.49, 51.93],
      [4.46, 51.93],
      [4.46, 51.91]
    ]]
  },
  "properties": {
    "datetime": "2023-05-15T00:00:00Z",
    "title": "Rotterdam Buildings LOD2",
    "description": "Building models in Level of Detail 2 for Rotterdam city center",
    "proj:epsg": 7415,
    "cj:encoding": "CityJSON",
    "cj:version": "2.0",
    "cj:city_objects": 1523,
    "cj:lods": ["2", "2.2"],
    "cj:co_types": ["Building", "BuildingPart"],
    "cj:attributes": [
      {
        "name": "yearOfConstruction",
        "type": "Number",
        "description": "Year built"
      },
      {
        "name": "function",
        "type": "String",
        "description": "Building function"
      }
    ],
    "cj:transform": {
      "scale": [0.001, 0.001, 0.001],
      "translate": [4629170.0, 5804690.0, 0.0]
    }
  },
  "assets": {
    "data": {
      "href": "./rotterdam_buildings_lod2.json",
      "type": "application/json",
      "title": "CityJSON data file",
      "roles": ["data"]
    }
  },
  "links": [
    {
      "rel": "self",
      "href": "./rotterdam_buildings_lod2_item.json"
    },
    {
      "rel": "parent",
      "href": "./collection.json"
    },
    {
      "rel": "collection",
      "href": "./collection.json"
    }
  ]
}
```

## STAC Collection Example

Complete example of a STAC Collection aggregating multiple CityJSON files:

```json
{
  "stac_version": "1.0.0",
  "stac_extensions": [
    "https://raw.githubusercontent.com/yourusername/cityjson-stac/main/extension.json",
    "https://stac-extensions.github.io/projection/v1.1.0/schema.json"
  ],
  "type": "Collection",
  "id": "rotterdam_3dcity_2023",
  "title": "Rotterdam 3D City Model 2023",
  "description": "Complete 3D city model of Rotterdam including buildings, roads, and terrain in multiple LODs",
  "license": "CC-BY-4.0",
  "extent": {
    "spatial": {
      "bbox": [[4.46, 51.91, -5.0, 4.49, 51.93, 100.0]]
    },
    "temporal": {
      "interval": [["2023-05-15T00:00:00Z", null]]
    }
  },
  "summaries": {
    "proj:epsg": [7415],
    "cj:encoding": ["CityJSON", "FlatCityBuf"],
    "cj:city_objects": {
      "min": 45,
      "max": 5000,
      "total": 125432
    },
    "cj:lods": ["0", "1", "2", "2.2", "3"],
    "cj:co_types": [
      "Building",
      "BuildingPart",
      "Road",
      "Railway",
      "TINRelief",
      "WaterBody"
    ]
  },
  "links": [
    {
      "rel": "self",
      "href": "./collection.json"
    },
    {
      "rel": "root",
      "href": "./catalog.json"
    },
    {
      "rel": "item",
      "href": "./items/rotterdam_buildings_lod2_item.json"
    },
    {
      "rel": "item",
      "href": "./items/rotterdam_terrain_item.json"
    }
  ]
}
```

## Integration with Other STAC Extensions

### Projection Extension

The CityJSON extension works well with the Projection extension:

```json
"proj:epsg": 7415,
"proj:wkt2": "...",
"proj:projjson": {...}
```

### File Extension

For describing file characteristics:

```json
"file:size": 15728640,
"file:checksum": "1220abcd..."
```

### Processing Extension

For derived datasets:

```json
"processing:level": "L2",
"processing:facility": "City GIS Lab",
"processing:software": {
  "cityjson-tools": "2.0.0"
}
```

## Best Practices

### 1. Bounding Box Representation
- STAC bbox should be 3D: `[xmin, ymin, zmin, xmax, ymax, zmax]`
- For collections, use the union of all item bboxes
- Ensure CRS consistency between bbox and geometry

### 2. Geometry Simplification
- STAC geometry should be 2D footprint (projected to WGS84)
- Simplify complex building footprints for better map display
- For items with many objects, use convex hull or envelope

### 3. Temporal Information
- Use `datetime` for single timestamp datasets
- Use `start_datetime` and `end_datetime` for temporal ranges
- Include reference date in `cj:metadata` for historical context

### 4. Asset Organization
- Primary data file should be in `data` asset
- Include thumbnails/previews if available
- Link to related documentation or schemas

### 5. Collection Organization
- One collection per thematic/geographic dataset
- Items should be homogeneous in quality and LOD
- Use subcollections for different areas or time periods

## Validation

### JSON Schema

The extension provides a JSON Schema for validation:

```bash
cityjson-stac validate <stac-file.json>
```

### Required Validations

1. Extension URL present in `stac_extensions`
2. All required `cj:*` properties present
3. Property types match specification
4. `cj:encoding` value is valid
5. `cj:lods` and `cj:co_types` are non-empty arrays
6. EPSG code is valid

## Extension Schema URL

```
https://raw.githubusercontent.com/[username]/cityjson-stac/main/stac-extension/schema.json
```

## Contributing

This extension is under development. Feedback and contributions welcome via GitHub issues and pull requests.

## Changelog

### Version 1.0.0 (Proposal)
- Initial extension specification
- Support for CityJSON, CityJSONSeq, and FlatCityBuf formats
- Item and Collection level properties
- Attribute schema definitions
