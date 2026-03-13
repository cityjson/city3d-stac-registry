//! STAC GeoParquet writer
//!
//! Encodes STAC Items as rows in a GeoParquet file following the
//! [STAC GeoParquet spec v1.1.0](https://github.com/radiantearth/stac-geoparquet-spec).

use crate::error::Result;
use crate::stac::models::{StacCollection, StacItem};
use arrow::array::{
    ArrayRef, BinaryBuilder, Float64Builder, ListBuilder, StringBuilder, StructBuilder,
    TimestampMillisecondBuilder, UInt64Builder,
};
use arrow::datatypes::{DataType, Field, Schema, TimeUnit};
use arrow::record_batch::RecordBatch;
use parquet::arrow::ArrowWriter;
use parquet::basic::Compression;
use parquet::file::properties::WriterProperties;
use std::collections::BTreeSet;
use std::path::Path;
use std::sync::Arc;

/// Convert a GeoJSON geometry (serde_json::Value) to ISO WKB bytes using geozero.
fn geojson_to_wkb(geometry: &serde_json::Value) -> Result<Vec<u8>> {
    use geozero::geojson::GeoJsonString;
    use geozero::wkb::WkbWriter;
    use geozero::GeozeroGeometry;

    let json_str = serde_json::to_string(geometry).map_err(|e| {
        crate::error::CityJsonStacError::StacError(format!("Failed to serialize geometry: {e}"))
    })?;
    let geojson = GeoJsonString(json_str);
    let mut wkb_out = Vec::new();
    let mut writer = WkbWriter::new(&mut wkb_out, geozero::wkb::WkbDialect::Wkb);
    geojson.process_geom(&mut writer).map_err(|e| {
        crate::error::CityJsonStacError::StacError(format!("WKB conversion failed: {e}"))
    })?;
    Ok(wkb_out)
}

/// Collect all unique property keys across items, excluding `datetime`.
fn collect_property_keys(items: &[StacItem]) -> Vec<String> {
    let mut keys = BTreeSet::new();
    for item in items {
        for key in item.properties.keys() {
            if key != "datetime" {
                keys.insert(key.clone());
            }
        }
    }
    keys.into_iter().collect()
}

/// Infer an Arrow DataType from a serde_json::Value.
fn infer_type(value: &serde_json::Value) -> DataType {
    match value {
        serde_json::Value::Bool(_) => DataType::Boolean,
        serde_json::Value::Number(n) => {
            if n.is_f64() {
                DataType::Float64
            } else {
                DataType::Int64
            }
        }
        serde_json::Value::String(_) => DataType::Utf8,
        serde_json::Value::Array(arr) => {
            let elem_type = arr
                .iter()
                .find(|v| !v.is_null())
                .map(infer_type)
                .unwrap_or(DataType::Utf8);
            DataType::List(Arc::new(Field::new("item", elem_type, true)))
        }
        _ => DataType::Utf8,
    }
}

/// Map known `city3d:*` properties to hardcoded Arrow types.
fn city3d_type(key: &str) -> Option<DataType> {
    match key {
        "city3d:lods" | "city3d:co_types" => Some(DataType::List(Arc::new(Field::new(
            "item",
            DataType::Utf8,
            true,
        )))),
        "city3d:city_objects" => Some(DataType::UInt64),
        "city3d:version" | "city3d:encoding" | "city3d:encoding_version" => Some(DataType::Utf8),
        "city3d:semantic_surfaces" | "city3d:textures" | "city3d:materials" => {
            Some(DataType::Boolean)
        }
        _ => None,
    }
}

/// Determine the Arrow DataType for a property key by scanning all items.
fn infer_property_type(key: &str, items: &[StacItem]) -> DataType {
    if let Some(dt) = city3d_type(key) {
        return dt;
    }
    // Infer from first non-null value
    for item in items {
        if let Some(val) = item.properties.get(key) {
            if !val.is_null() {
                return infer_type(val);
            }
        }
    }
    DataType::Utf8
}

// ─── Link / Asset struct helpers ────────────────────────────────────

fn link_fields() -> Vec<Field> {
    vec![
        Field::new("href", DataType::Utf8, false),
        Field::new("rel", DataType::Utf8, false),
        Field::new("type", DataType::Utf8, true),
        Field::new("title", DataType::Utf8, true),
    ]
}

fn asset_value_fields() -> Vec<Field> {
    vec![
        Field::new("href", DataType::Utf8, false),
        Field::new("title", DataType::Utf8, true),
        Field::new("description", DataType::Utf8, true),
        Field::new("type", DataType::Utf8, true),
        // Roles serialized as JSON string to avoid nested List inside Struct
        Field::new("roles", DataType::Utf8, true),
    ]
}

/// Collect all unique asset keys across items.
fn collect_asset_keys(items: &[StacItem]) -> Vec<String> {
    let mut keys = BTreeSet::new();
    for item in items {
        for key in item.assets.keys() {
            keys.insert(key.clone());
        }
    }
    keys.into_iter().collect()
}

fn bbox_fields() -> Vec<Field> {
    vec![
        Field::new("xmin", DataType::Float64, false),
        Field::new("ymin", DataType::Float64, false),
        Field::new("zmin", DataType::Float64, true),
        Field::new("xmax", DataType::Float64, false),
        Field::new("ymax", DataType::Float64, false),
        Field::new("zmax", DataType::Float64, true),
    ]
}

// ─── Schema construction ────────────────────────────────────────────

fn build_arrow_schema(items: &[StacItem]) -> Schema {
    let mut fields = vec![
        Field::new("id", DataType::Utf8, false),
        Field::new("geometry", DataType::Binary, true),
        Field::new("bbox", DataType::Struct(bbox_fields().into()), true),
        Field::new(
            "datetime",
            DataType::Timestamp(TimeUnit::Millisecond, Some("UTC".into())),
            true,
        ),
        Field::new(
            "stac_extensions",
            DataType::List(Arc::new(Field::new("item", DataType::Utf8, true))),
            true,
        ),
        Field::new(
            "links",
            DataType::List(Arc::new(Field::new(
                "item",
                DataType::Struct(link_fields().into()),
                true,
            ))),
            true,
        ),
        Field::new("collection", DataType::Utf8, true),
    ];

    // Dynamic property columns
    let prop_keys = collect_property_keys(items);
    for key in &prop_keys {
        let dt = infer_property_type(key, items);
        fields.push(Field::new(key.as_str(), dt, true));
    }

    // Assets as a struct with one sub-struct per unique asset key
    let asset_keys = collect_asset_keys(items);
    if !asset_keys.is_empty() {
        let asset_struct_fields: Vec<Field> = asset_keys
            .iter()
            .map(|k| {
                Field::new(
                    k.as_str(),
                    DataType::Struct(asset_value_fields().into()),
                    true,
                )
            })
            .collect();
        fields.push(Field::new(
            "assets",
            DataType::Struct(asset_struct_fields.into()),
            true,
        ));
    }

    Schema::new(fields)
}

// ─── RecordBatch construction ───────────────────────────────────────

fn parse_datetime_ms(s: &str) -> Option<i64> {
    chrono::DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|dt| dt.timestamp_millis())
}

fn build_record_batch(items: &[StacItem], schema: &Schema) -> Result<RecordBatch> {
    let n = items.len();
    let prop_keys = collect_property_keys(items);
    let asset_keys = collect_asset_keys(items);

    // ── Fixed column builders ──

    let mut id_builder = StringBuilder::with_capacity(n, n * 32);
    let mut geom_builder = BinaryBuilder::with_capacity(n, n * 256);

    // bbox struct
    let mut bbox_xmin = Float64Builder::with_capacity(n);
    let mut bbox_ymin = Float64Builder::with_capacity(n);
    let mut bbox_zmin = Float64Builder::with_capacity(n);
    let mut bbox_xmax = Float64Builder::with_capacity(n);
    let mut bbox_ymax = Float64Builder::with_capacity(n);
    let mut bbox_zmax = Float64Builder::with_capacity(n);

    let mut datetime_builder = TimestampMillisecondBuilder::with_capacity(n);

    // links: List<Struct{href, rel, type, title}>
    let mut links_builder = ListBuilder::new(StructBuilder::from_fields(link_fields(), n));

    let mut collection_builder = StringBuilder::with_capacity(n, n * 32);

    // ── Dynamic property builders ──
    // We'll collect values as JSON and convert at the end for simplicity with mixed types
    let mut prop_columns: Vec<Vec<Option<serde_json::Value>>> =
        vec![Vec::with_capacity(n); prop_keys.len()];

    // ── Asset builders ──
    // For each asset key, build a struct {href, title, description, type, roles}
    let asset_fields_template = asset_value_fields();
    let mut asset_builders: Vec<StructBuilder> = asset_keys
        .iter()
        .map(|_| StructBuilder::from_fields(asset_fields_template.clone(), n))
        .collect();

    // ── Populate ──

    for item in items {
        // id
        id_builder.append_value(&item.id);

        // geometry → WKB
        if let Some(geom) = &item.geometry {
            match geojson_to_wkb(geom) {
                Ok(wkb) => geom_builder.append_value(&wkb),
                Err(_) => geom_builder.append_null(),
            }
        } else {
            geom_builder.append_null();
        }

        // bbox
        if let Some(bb) = &item.bbox {
            bbox_xmin.append_value(*bb.first().unwrap_or(&0.0));
            bbox_ymin.append_value(*bb.get(1).unwrap_or(&0.0));
            bbox_xmax.append_value(*bb.get(3).unwrap_or(bb.get(2).unwrap_or(&0.0)));
            bbox_ymax.append_value(*bb.get(4).unwrap_or(bb.get(3).unwrap_or(&0.0)));
            // 3D bbox has 6 elements: [xmin, ymin, zmin, xmax, ymax, zmax]
            if bb.len() == 6 {
                bbox_zmin.append_option(Some(bb[2]));
                bbox_zmax.append_option(Some(bb[5]));
            } else {
                bbox_zmin.append_null();
                bbox_zmax.append_null();
            }
        } else {
            bbox_xmin.append_null();
            bbox_ymin.append_null();
            bbox_zmin.append_null();
            bbox_xmax.append_null();
            bbox_ymax.append_null();
            bbox_zmax.append_null();
        }

        // datetime
        let dt_ms = item
            .properties
            .get("datetime")
            .and_then(|v| v.as_str())
            .and_then(parse_datetime_ms);
        match dt_ms {
            Some(ms) => datetime_builder.append_value(ms),
            None => datetime_builder.append_null(),
        }

        // links
        let link_struct = links_builder.values();
        for link in &item.links {
            link_struct
                .field_builder::<StringBuilder>(0)
                .unwrap()
                .append_value(&link.href);
            link_struct
                .field_builder::<StringBuilder>(1)
                .unwrap()
                .append_value(&link.rel);
            match &link.link_type {
                Some(t) => link_struct
                    .field_builder::<StringBuilder>(2)
                    .unwrap()
                    .append_value(t),
                None => link_struct
                    .field_builder::<StringBuilder>(2)
                    .unwrap()
                    .append_null(),
            }
            match &link.title {
                Some(t) => link_struct
                    .field_builder::<StringBuilder>(3)
                    .unwrap()
                    .append_value(t),
                None => link_struct
                    .field_builder::<StringBuilder>(3)
                    .unwrap()
                    .append_null(),
            }
            link_struct.append(true);
        }
        links_builder.append(true);

        // collection (from properties if present, otherwise from first link with rel=collection)
        let coll_id = item
            .links
            .iter()
            .find(|l| l.rel == "collection")
            .and_then(|l| l.title.as_deref());
        match coll_id {
            Some(c) => collection_builder.append_value(c),
            None => collection_builder.append_null(),
        }

        // Dynamic properties
        for (i, key) in prop_keys.iter().enumerate() {
            prop_columns[i].push(item.properties.get(key).cloned());
        }

        // Assets
        for (i, asset_key) in asset_keys.iter().enumerate() {
            let builder = &mut asset_builders[i];
            if let Some(asset) = item.assets.get(asset_key) {
                builder
                    .field_builder::<StringBuilder>(0)
                    .unwrap()
                    .append_value(&asset.href);
                match &asset.title {
                    Some(t) => builder
                        .field_builder::<StringBuilder>(1)
                        .unwrap()
                        .append_value(t),
                    None => builder
                        .field_builder::<StringBuilder>(1)
                        .unwrap()
                        .append_null(),
                }
                match &asset.description {
                    Some(d) => builder
                        .field_builder::<StringBuilder>(2)
                        .unwrap()
                        .append_value(d),
                    None => builder
                        .field_builder::<StringBuilder>(2)
                        .unwrap()
                        .append_null(),
                }
                match &asset.media_type {
                    Some(t) => builder
                        .field_builder::<StringBuilder>(3)
                        .unwrap()
                        .append_value(t),
                    None => builder
                        .field_builder::<StringBuilder>(3)
                        .unwrap()
                        .append_null(),
                }
                // roles (serialized as JSON string)
                match &asset.roles {
                    Some(roles) => {
                        let roles_json = serde_json::to_string(roles).unwrap_or_default();
                        builder
                            .field_builder::<StringBuilder>(4)
                            .unwrap()
                            .append_value(&roles_json);
                    }
                    None => {
                        builder
                            .field_builder::<StringBuilder>(4)
                            .unwrap()
                            .append_null();
                    }
                }
                builder.append(true);
            } else {
                // Null asset
                builder
                    .field_builder::<StringBuilder>(0)
                    .unwrap()
                    .append_null();
                builder
                    .field_builder::<StringBuilder>(1)
                    .unwrap()
                    .append_null();
                builder
                    .field_builder::<StringBuilder>(2)
                    .unwrap()
                    .append_null();
                builder
                    .field_builder::<StringBuilder>(3)
                    .unwrap()
                    .append_null();
                builder
                    .field_builder::<StringBuilder>(4)
                    .unwrap()
                    .append_null();
                builder.append(false);
            }
        }
    }

    // ── Build arrays ──

    let mut columns: Vec<ArrayRef> = Vec::new();

    columns.push(Arc::new(id_builder.finish()));
    columns.push(Arc::new(geom_builder.finish()));

    // bbox struct (with nullable zmin/zmax)
    let bbox_struct = arrow::array::StructArray::new(
        bbox_fields().into(),
        vec![
            Arc::new(bbox_xmin.finish()) as ArrayRef,
            Arc::new(bbox_ymin.finish()) as ArrayRef,
            Arc::new(bbox_zmin.finish()) as ArrayRef,
            Arc::new(bbox_xmax.finish()) as ArrayRef,
            Arc::new(bbox_ymax.finish()) as ArrayRef,
            Arc::new(bbox_zmax.finish()) as ArrayRef,
        ],
        None,
    );
    columns.push(Arc::new(bbox_struct));

    // datetime
    columns.push(Arc::new(datetime_builder.finish().with_timezone("UTC")));

    // stac_extensions
    {
        let mut ext_builder = ListBuilder::new(StringBuilder::new());
        for item in items {
            for ext in &item.stac_extensions {
                ext_builder.values().append_value(ext);
            }
            ext_builder.append(true);
        }
        columns.push(Arc::new(ext_builder.finish()));
    }

    // links
    columns.push(Arc::new(links_builder.finish()));

    // collection
    columns.push(Arc::new(collection_builder.finish()));

    // Dynamic property columns
    for (i, key) in prop_keys.iter().enumerate() {
        let dt = infer_property_type(key, items);
        let col = json_values_to_array(&prop_columns[i], &dt, n);
        columns.push(col);
    }

    // Assets struct
    if !asset_keys.is_empty() {
        let asset_arrays: Vec<(Arc<Field>, ArrayRef)> = asset_keys
            .iter()
            .zip(asset_builders.iter_mut())
            .map(|(key, builder)| {
                let arr = builder.finish();
                let field = Arc::new(Field::new(
                    key.as_str(),
                    DataType::Struct(asset_value_fields().into()),
                    true,
                ));
                (field, Arc::new(arr) as ArrayRef)
            })
            .collect();
        let assets_struct = arrow::array::StructArray::from(asset_arrays);
        columns.push(Arc::new(assets_struct));
    }

    Ok(RecordBatch::try_new(Arc::new(schema.clone()), columns)?)
}

/// Convert a slice of optional JSON values to an Arrow array of the given type.
fn json_values_to_array(
    values: &[Option<serde_json::Value>],
    data_type: &DataType,
    _capacity: usize,
) -> ArrayRef {
    match data_type {
        DataType::Utf8 => {
            let mut b = StringBuilder::new();
            for v in values {
                match v {
                    Some(serde_json::Value::String(s)) => b.append_value(s),
                    Some(val) if !val.is_null() => b.append_value(val.to_string()),
                    _ => b.append_null(),
                }
            }
            Arc::new(b.finish())
        }
        DataType::Int64 => {
            let mut b = arrow::array::Int64Builder::new();
            for v in values {
                match v {
                    Some(serde_json::Value::Number(n)) => {
                        b.append_option(n.as_i64());
                    }
                    _ => b.append_null(),
                }
            }
            Arc::new(b.finish())
        }
        DataType::UInt64 => {
            let mut b = UInt64Builder::new();
            for v in values {
                match v {
                    Some(serde_json::Value::Number(n)) => {
                        b.append_option(n.as_u64());
                    }
                    _ => b.append_null(),
                }
            }
            Arc::new(b.finish())
        }
        DataType::Float64 => {
            let mut b = Float64Builder::new();
            for v in values {
                match v {
                    Some(serde_json::Value::Number(n)) => {
                        b.append_option(n.as_f64());
                    }
                    _ => b.append_null(),
                }
            }
            Arc::new(b.finish())
        }
        DataType::Boolean => {
            let mut b = arrow::array::BooleanBuilder::new();
            for v in values {
                match v {
                    Some(serde_json::Value::Bool(bv)) => b.append_value(*bv),
                    _ => b.append_null(),
                }
            }
            Arc::new(b.finish())
        }
        DataType::List(_) => {
            // List<Utf8> — serialize each element as string
            let mut b = ListBuilder::new(StringBuilder::new());
            for v in values {
                match v {
                    Some(serde_json::Value::Array(arr)) => {
                        for elem in arr {
                            match elem {
                                serde_json::Value::String(s) => b.values().append_value(s),
                                other if !other.is_null() => {
                                    b.values().append_value(other.to_string())
                                }
                                _ => b.values().append_null(),
                            }
                        }
                        b.append(true);
                    }
                    _ => b.append(false),
                }
            }
            Arc::new(b.finish())
        }
        _ => {
            // Fallback: stringify everything
            let mut b = StringBuilder::new();
            for v in values {
                match v {
                    Some(val) if !val.is_null() => b.append_value(val.to_string()),
                    _ => b.append_null(),
                }
            }
            Arc::new(b.finish())
        }
    }
}

// ─── GeoParquet metadata ────────────────────────────────────────────

fn geo_metadata_json() -> String {
    serde_json::json!({
        "version": "1.1.0",
        "primary_column": "geometry",
        "columns": {
            "geometry": {
                "encoding": "WKB",
                "geometry_types": ["Polygon"],
                "crs": {
                    "$schema": "https://proj.org/schemas/v0.7/projjson.schema.json",
                    "type": "GeographicCRS",
                    "name": "WGS 84",
                    "datum": {
                        "type": "GeodeticReferenceFrame",
                        "name": "World Geodetic System 1984",
                        "ellipsoid": {
                            "name": "WGS 84",
                            "semi_major_axis": 6378137,
                            "inverse_flattening": 298.257223563
                        }
                    },
                    "coordinate_system": {
                        "subtype": "ellipsoidal",
                        "axis": [
                            {"name": "Geodetic latitude", "abbreviation": "Lat", "direction": "north", "unit": "degree"},
                            {"name": "Geodetic longitude", "abbreviation": "Lon", "direction": "east", "unit": "degree"}
                        ]
                    },
                    "id": {"authority": "EPSG", "code": 4326}
                },
                "bbox": null
            }
        }
    })
    .to_string()
}

fn stac_geoparquet_metadata_json(collection: &StacCollection) -> String {
    let collection_json = serde_json::to_value(collection).unwrap_or_default();
    serde_json::json!({
        "version": "1.1.0",
        "collections": {
            collection.id.clone(): collection_json
        }
    })
    .to_string()
}

// ─── Public API ─────────────────────────────────────────────────────

/// Write STAC items as a GeoParquet file.
///
/// The output file will contain one row per item with GeoParquet 1.1.0
/// and STAC GeoParquet 1.1.0 metadata embedded in the Parquet file metadata.
pub fn write_geoparquet(
    items: &[StacItem],
    collection: &StacCollection,
    output_path: &Path,
) -> Result<()> {
    if items.is_empty() {
        return Ok(());
    }

    let schema = build_arrow_schema(items);
    let batch = build_record_batch(items, &schema)?;

    let props = WriterProperties::builder()
        .set_compression(Compression::SNAPPY)
        .set_key_value_metadata(Some(vec![
            parquet::format::KeyValue {
                key: "geo".to_string(),
                value: Some(geo_metadata_json()),
            },
            parquet::format::KeyValue {
                key: "stac-geoparquet".to_string(),
                value: Some(stac_geoparquet_metadata_json(collection)),
            },
        ]))
        .build();

    let file = std::fs::File::create(output_path)?;
    let mut writer = ArrowWriter::try_new(file, Arc::new(schema), Some(props))?;
    writer.write(&batch)?;
    writer.close()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stac::models::{Asset, Extent, Link, SpatialExtent, TemporalExtent};
    use std::collections::HashMap;

    fn make_test_item(id: &str, bbox: Vec<f64>) -> StacItem {
        let geometry = serde_json::json!({
            "type": "Polygon",
            "coordinates": [[
                [bbox[0], bbox[1]],
                [bbox[3], bbox[1]],
                [bbox[3], bbox[4]],
                [bbox[0], bbox[4]],
                [bbox[0], bbox[1]],
            ]]
        });
        let mut properties = HashMap::new();
        properties.insert(
            "datetime".to_string(),
            serde_json::Value::String("2024-01-15T12:00:00Z".to_string()),
        );
        properties.insert(
            "city3d:encoding".to_string(),
            serde_json::Value::String("CityJSON".to_string()),
        );
        properties.insert("city3d:lods".to_string(), serde_json::json!(["1.2", "2.2"]));
        properties.insert("city3d:city_objects".to_string(), serde_json::json!(42));

        let mut assets = HashMap::new();
        assets.insert(
            "data".to_string(),
            Asset::new("./data.city.json")
                .with_type("application/city+json")
                .with_roles(vec!["data".to_string()]),
        );

        StacItem {
            stac_version: "1.1.0".to_string(),
            stac_extensions: vec![
                "https://cityjson.github.io/stac-city3d/v0.1.0/schema.json".to_string()
            ],
            item_type: "Feature".to_string(),
            id: id.to_string(),
            bbox: Some(bbox),
            geometry: Some(geometry),
            properties,
            assets,
            links: vec![Link::new("self", format!("./{id}_item.json"))],
        }
    }

    fn make_test_collection() -> StacCollection {
        StacCollection {
            stac_version: "1.1.0".to_string(),
            stac_extensions: vec![],
            collection_type: "Collection".to_string(),
            id: "test-collection".to_string(),
            title: Some("Test Collection".to_string()),
            description: Some("A test collection".to_string()),
            license: "proprietary".to_string(),
            keywords: None,
            providers: None,
            extent: Extent {
                spatial: SpatialExtent {
                    bbox: vec![vec![0.0, 0.0, 0.0, 10.0, 10.0, 100.0]],
                },
                temporal: TemporalExtent::default(),
            },
            summaries: None,
            links: vec![],
            assets: None,
        }
    }

    #[test]
    fn test_geojson_polygon_to_wkb() {
        let geom = serde_json::json!({
            "type": "Polygon",
            "coordinates": [[
                [4.0, 52.0],
                [5.0, 52.0],
                [5.0, 53.0],
                [4.0, 53.0],
                [4.0, 52.0],
            ]]
        });
        let wkb = geojson_to_wkb(&geom).unwrap();
        // Byte order (1) + type (4) + num_rings (4) + num_points (4) + 5 points * 16 bytes
        assert_eq!(wkb.len(), 1 + 4 + 4 + 4 + 5 * 16);
        assert_eq!(wkb[0], 0x01); // little-endian
        assert_eq!(u32::from_le_bytes([wkb[1], wkb[2], wkb[3], wkb[4]]), 3); // Polygon
    }

    #[test]
    fn test_build_schema() {
        let items = vec![make_test_item("a", vec![0.0, 0.0, 0.0, 10.0, 10.0, 100.0])];
        let schema = build_arrow_schema(&items);

        let field_names: Vec<&str> = schema.fields().iter().map(|f| f.name().as_str()).collect();
        assert!(field_names.contains(&"id"));
        assert!(field_names.contains(&"geometry"));
        assert!(field_names.contains(&"bbox"));
        assert!(field_names.contains(&"datetime"));
        assert!(field_names.contains(&"city3d:encoding"));
        assert!(field_names.contains(&"city3d:lods"));
        assert!(field_names.contains(&"assets"));
    }

    #[test]
    fn test_write_single_item() {
        let items = vec![make_test_item(
            "item-1",
            vec![4.0, 52.0, 0.0, 5.0, 53.0, 100.0],
        )];
        let collection = make_test_collection();

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("items.parquet");
        write_geoparquet(&items, &collection, &path).unwrap();

        assert!(path.exists());
        assert!(std::fs::metadata(&path).unwrap().len() > 0);

        // Read back and verify
        let file = std::fs::File::open(&path).unwrap();
        let reader = parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder::try_new(file)
            .unwrap()
            .build()
            .unwrap();
        let batches: Vec<RecordBatch> = reader.collect::<std::result::Result<Vec<_>, _>>().unwrap();
        assert_eq!(batches.len(), 1);
        assert_eq!(batches[0].num_rows(), 1);

        // Check id column
        let id_col = batches[0]
            .column_by_name("id")
            .unwrap()
            .as_any()
            .downcast_ref::<arrow::array::StringArray>()
            .unwrap();
        assert_eq!(id_col.value(0), "item-1");
    }

    #[test]
    fn test_write_multiple_items() {
        let items = vec![
            make_test_item("item-1", vec![4.0, 52.0, 0.0, 5.0, 53.0, 100.0]),
            make_test_item("item-2", vec![5.0, 52.0, 0.0, 6.0, 53.0, 50.0]),
            make_test_item("item-3", vec![6.0, 52.0, 0.0, 7.0, 53.0, 75.0]),
        ];
        let collection = make_test_collection();

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("items.parquet");
        write_geoparquet(&items, &collection, &path).unwrap();

        let file = std::fs::File::open(&path).unwrap();
        let reader = parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder::try_new(file)
            .unwrap()
            .build()
            .unwrap();
        let batches: Vec<RecordBatch> = reader.collect::<std::result::Result<Vec<_>, _>>().unwrap();
        assert_eq!(batches[0].num_rows(), 3);
    }

    #[test]
    fn test_geoparquet_metadata() {
        let items = vec![make_test_item(
            "item-1",
            vec![4.0, 52.0, 0.0, 5.0, 53.0, 100.0],
        )];
        let collection = make_test_collection();

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("items.parquet");
        write_geoparquet(&items, &collection, &path).unwrap();

        let file = std::fs::File::open(&path).unwrap();
        let reader = parquet::file::reader::SerializedFileReader::new(file).unwrap();
        use parquet::file::reader::FileReader;
        let metadata = reader
            .metadata()
            .file_metadata()
            .key_value_metadata()
            .unwrap();

        let geo_kv = metadata.iter().find(|kv| kv.key == "geo").unwrap();
        let geo_json: serde_json::Value =
            serde_json::from_str(geo_kv.value.as_deref().unwrap()).unwrap();
        assert_eq!(geo_json["version"], "1.1.0");
        assert_eq!(geo_json["primary_column"], "geometry");

        let stac_kv = metadata
            .iter()
            .find(|kv| kv.key == "stac-geoparquet")
            .unwrap();
        let stac_json: serde_json::Value =
            serde_json::from_str(stac_kv.value.as_deref().unwrap()).unwrap();
        assert_eq!(stac_json["version"], "1.1.0");
        assert!(stac_json["collections"]["test-collection"].is_object());
    }

    #[test]
    fn test_datetime_as_timestamp() {
        let items = vec![make_test_item(
            "item-1",
            vec![4.0, 52.0, 0.0, 5.0, 53.0, 100.0],
        )];
        let collection = make_test_collection();

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("items.parquet");
        write_geoparquet(&items, &collection, &path).unwrap();

        let file = std::fs::File::open(&path).unwrap();
        let reader = parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder::try_new(file)
            .unwrap()
            .build()
            .unwrap();
        let batches: Vec<RecordBatch> = reader.collect::<std::result::Result<Vec<_>, _>>().unwrap();

        let dt_col = batches[0]
            .column_by_name("datetime")
            .unwrap()
            .as_any()
            .downcast_ref::<arrow::array::TimestampMillisecondArray>()
            .unwrap();
        // 2024-01-15T12:00:00Z → 1705320000000 ms
        assert_eq!(dt_col.value(0), 1705320000000);
    }

    #[test]
    fn test_bbox_struct() {
        let items = vec![make_test_item(
            "item-1",
            vec![4.0, 52.0, 10.0, 5.0, 53.0, 100.0],
        )];
        let collection = make_test_collection();

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("items.parquet");
        write_geoparquet(&items, &collection, &path).unwrap();

        let file = std::fs::File::open(&path).unwrap();
        let reader = parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder::try_new(file)
            .unwrap()
            .build()
            .unwrap();
        let batches: Vec<RecordBatch> = reader.collect::<std::result::Result<Vec<_>, _>>().unwrap();

        let bbox_col = batches[0]
            .column_by_name("bbox")
            .unwrap()
            .as_any()
            .downcast_ref::<arrow::array::StructArray>()
            .unwrap();
        let xmin = bbox_col
            .column_by_name("xmin")
            .unwrap()
            .as_any()
            .downcast_ref::<arrow::array::Float64Array>()
            .unwrap();
        assert!((xmin.value(0) - 4.0).abs() < f64::EPSILON);

        let zmax = bbox_col
            .column_by_name("zmax")
            .unwrap()
            .as_any()
            .downcast_ref::<arrow::array::Float64Array>()
            .unwrap();
        assert!((zmax.value(0) - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_empty_items() {
        let collection = make_test_collection();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("items.parquet");
        write_geoparquet(&[], &collection, &path).unwrap();
        assert!(!path.exists()); // No file written for empty items
    }
}
