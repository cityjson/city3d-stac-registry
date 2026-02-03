# CityJSON STAC Extension Specification

**Extension Name:** CityJSON  
**Extension Prefix:** `cj`  
**Extension Version:** 1.0.0  
**Scope:** Item, Collection  
**Maturity:** Proposal

## Purpose

This extension defines properties for STAC Items and Collections describing 3D city model datasets in CityJSON and related formats. All extension properties use the `cj:` prefix.

---

## Item-Level Properties

Properties in the STAC Item `properties` object:

| Field Name        | Type          | Required | Description                                      |
| ----------------- | ------------- | -------- | ------------------------------------------------ |
| `cj:encoding`     | string        | Yes      | Format: `CityJSON`, `CityJSONSeq`, `FlatCityBuf` |
| `cj:version`      | string        | Yes      | CityJSON version (e.g., "2.0", "1.1")            |
| `cj:city_objects` | integer       | Yes      | Number of city objects                           |
| `cj:lods`         | array[string] | Yes      | Available levels of detail (e.g., ["1", "2"])    |
| `cj:co_types`     | array[string] | Yes      | City object types (e.g., ["Building", "Road"])   |
| `cj:attributes`   | array[object] | No       | Attribute schema definitions                     |
| `cj:transform`    | object        | No       | Coordinate transformation parameters             |
| `cj:metadata`     | object        | No       | Additional CityJSON metadata                     |
| `cj:extensions`   | array[string] | No       | CityJSON extension schema URLs (ADEs)            |

---

## Collection-Level Properties

Properties in the STAC Collection `summaries` object:

| Field Name        | Type          | Required | Description                             |
| ----------------- | ------------- | -------- | --------------------------------------- |
| `cj:encoding`     | array[string] | Yes      | All encoding formats in collection      |
| `cj:city_objects` | object        | Yes      | Statistics: `{min, max, total}`         |
| `cj:lods`         | array[string] | Yes      | All LODs across collection              |
| `cj:co_types`     | array[string] | Yes      | All city object types across collection |
| `cj:extensions`   | array[string] | No       | All extension URLs across collection    |

---

## Property Details

### `cj:encoding`

Encoding format of the 3D city model data.

**Allowed Values:**

- `"CityJSON"` - Standard CityJSON (.json)
- `"CityJSONSeq"` - CityJSON Text Sequences (.jsonl)
- `"FlatCityBuf"` - FlatBuffers-based format (.fcb)

### `cj:version`

CityJSON specification version. Pattern: semantic version (e.g., "2.0", "1.1").

### `cj:city_objects`

**Item:** Integer count of city objects.

**Collection:** Statistics object:

```json
{
  "min": 45,
  "max": 5000,
  "total": 125432
}
```

### `cj:lods`

Levels of Detail available. Supports decimal values (e.g., "2.2").

```json
"cj:lods": ["1", "2", "2.2", "3"]
```

### `cj:co_types`

CityJSON object types present.

**Common Types:** Building, BuildingPart, Road, Railway, TINRelief, WaterBody, PlantCover, LandUse, Bridge, Tunnel, GenericCityObject

```json
"cj:co_types": ["Building", "BuildingPart", "TINRelief"]
```

### `cj:attributes`

Attribute schema definitions.

| Property    | Type    | Required | Description                                  |
| ----------- | ------- | -------- | -------------------------------------------- |
| name        | string  | Yes      | Attribute name                               |
| type        | string  | Yes      | String, Number, Boolean, Date, Array, Object |
| description | string  | No       | Human-readable description                   |
| required    | boolean | No       | Whether always present                       |

```json
"cj:attributes": [
  { "name": "yearOfConstruction", "type": "Number" },
  { "name": "function", "type": "String", "description": "Building usage" }
]
```

### `cj:transform`

Coordinate transformation for vertex compression.

| Property  | Type          | Description           |
| --------- | ------------- | --------------------- |
| scale     | array[number] | Scale factors [x,y,z] |
| translate | array[number] | Offsets [x,y,z]       |

```json
"cj:transform": {
  "scale": [0.001, 0.001, 0.001],
  "translate": [4629170.0, 5804690.0, 0.0]
}
```

### `cj:metadata`

Free-form additional CityJSON metadata.

```json
"cj:metadata": {
  "referenceDate": "2023-05-15",
  "dataSource": "LiDAR survey 2023"
}
```

### `cj:extensions`

CityJSON Extension schema URLs (Application Domain Extensions).

CityJSON Extensions allow extending the core data model with:

- New properties at the root level
- New attributes for existing City Objects
- New semantic objects
- New City Object types (prefixed with "+")

```json
"cj:extensions": [
  "https://www.cityjson.org/extensions/noise.ext.json",
  "https://3dbag.nl/extensions/3dbag.ext.json"
]
```

---

## Examples

### STAC Item

```json
{
  "stac_version": "1.0.0",
  "stac_extensions": [
    "https://raw.githubusercontent.com/cityjson/cityjson-stac/main/stac-extension/schema.json",
    "https://stac-extensions.github.io/projection/v1.1.0/schema.json"
  ],
  "type": "Feature",
  "id": "rotterdam_buildings_lod2",
  "bbox": [4.46, 51.91, -5.0, 4.49, 51.93, 100.0],
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
    "cj:encoding": "CityJSON",
    "cj:version": "2.0",
    "cj:city_objects": 1523,
    "cj:lods": ["2", "2.2"],
    "cj:co_types": ["Building", "BuildingPart"],
    "cj:transform": {
      "scale": [0.001, 0.001, 0.001],
      "translate": [4629170.0, 5804690.0, 0.0]
    }
  },
  "assets": {
    "data": {
      "href": "./rotterdam_buildings_lod2.json",
      "type": "application/json",
      "roles": ["data"]
    }
  },
  "links": [
    { "rel": "self", "href": "./rotterdam_buildings_lod2_item.json" },
    { "rel": "collection", "href": "./collection.json" }
  ]
}
```

### STAC Collection

```json
{
  "stac_version": "1.0.0",
  "stac_extensions": [
    "https://raw.githubusercontent.com/cityjson/cityjson-stac/main/stac-extension/schema.json"
  ],
  "type": "Collection",
  "id": "rotterdam_3dcity_2023",
  "title": "Rotterdam 3D City Model 2023",
  "description": "3D city model with buildings and terrain in multiple LODs",
  "license": "CC-BY-4.0",
  "extent": {
    "spatial": { "bbox": [[4.42, 51.88, -5.0, 4.6, 51.98, 120.5]] },
    "temporal": { "interval": [["2023-05-15T00:00:00Z", null]] }
  },
  "summaries": {
    "proj:epsg": [7415],
    "cj:encoding": ["CityJSON", "FlatCityBuf"],
    "cj:city_objects": { "min": 45, "max": 5000, "total": 125432 },
    "cj:lods": ["0", "1", "2", "3"],
    "cj:co_types": ["Building", "BuildingPart", "TINRelief", "WaterBody"]
  },
  "links": [
    { "rel": "self", "href": "./collection.json" },
    { "rel": "item", "href": "./items/buildings_item.json" }
  ]
}
```

---

## Compatible STAC Extensions

- **[Projection Extension](https://stac-extensions.github.io/projection/)**: `proj:epsg`, `proj:wkt2`
- **[File Extension](https://stac-extensions.github.io/file/)**: `file:size`, `file:checksum`

---

## Best Practices

1. **3D Bounding Box**: Use 6-element bbox `[xmin, ymin, zmin, xmax, ymax, zmax]`
2. **Geometry**: Provide 2D footprint polygon in WGS84 for map display
3. **Temporal**: Use `datetime` for single timestamp, or `start/end_datetime` for ranges
4. **Assets**: Include primary data file with `"roles": ["data"]`

---

## Schema URL

```
https://raw.githubusercontent.com/cityjson/cityjson-stac/main/stac-extension/schema.json
```

---

## Changelog

### v1.0.0 (Proposal)

- Initial specification
- Support for CityJSON, CityJSONSeq, FlatCityBuf formats
- Item and Collection level properties
