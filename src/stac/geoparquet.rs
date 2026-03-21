//! STAC GeoParquet writer
//!
//! Uses the upstream `stac` crate's geoparquet module to encode STAC Items
//! as rows in a GeoParquet file following the stac-geoparquet spec.

use crate::error::Result;
use std::path::Path;

/// Write STAC items as a GeoParquet file.
///
/// The output file will contain one row per item with GeoParquet and
/// stac-geoparquet metadata embedded in the Parquet file metadata.
/// Collection metadata is also embedded.
pub fn write_geoparquet(
    items: &[stac::Item],
    collection: &stac::Collection,
    output_path: &Path,
) -> Result<()> {
    if items.is_empty() {
        return Ok(());
    }

    let file = std::fs::File::create(output_path)?;
    let writer_options = stac::geoparquet::WriterOptions::new()
        .with_compression(stac::geoparquet::Compression::SNAPPY);

    stac::geoparquet::WriterBuilder::new(file)
        .writer_options(writer_options)
        .build(items.to_vec())
        .map_err(|e| {
            crate::error::CityJsonStacError::StacError(format!("GeoParquet encode error: {e}"))
        })?
        .add_collection(collection.clone())
        .map_err(|e| {
            crate::error::CityJsonStacError::StacError(format!("GeoParquet collection error: {e}"))
        })?
        .finish()
        .map_err(|e| {
            crate::error::CityJsonStacError::StacError(format!("GeoParquet write error: {e}"))
        })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_item(id: &str, bbox: Vec<f64>) -> stac::Item {
        let mut item = stac::Item::new(id);
        item.bbox = Some(bbox.try_into().unwrap());

        let bb: Vec<f64> = item.bbox.unwrap().into();
        let geometry = serde_json::json!({
            "type": "Polygon",
            "coordinates": [[
                [bb[0], bb[1]],
                [bb[3], bb[1]],
                [bb[3], bb[4]],
                [bb[0], bb[4]],
                [bb[0], bb[1]],
            ]]
        });
        item.geometry = serde_json::from_value(geometry).ok();

        item.properties.datetime = Some("2024-01-15T12:00:00Z".parse().unwrap());
        item.properties
            .additional_fields
            .insert("city3d:lods".to_string(), serde_json::json!(["1.2", "2.2"]));
        item.properties
            .additional_fields
            .insert("city3d:city_objects".to_string(), serde_json::json!(42));

        let mut asset = stac::Asset::new("./data.city.json");
        asset.r#type = Some("application/city+json".to_string());
        asset.roles = vec!["data".to_string()];
        item.assets.insert("data".to_string(), asset);

        item.extensions =
            vec!["https://cityjson.github.io/stac-city3d/v0.1.0/schema.json".to_string()];

        item.links
            .push(stac::Link::self_(format!("./{id}_item.json")));

        item
    }

    fn make_test_collection() -> stac::Collection {
        let mut collection = stac::Collection::new("test-collection", "A test collection");
        collection.title = Some("Test Collection".to_string());
        collection.license = "proprietary".to_string();
        collection.extent.spatial.bbox = vec![stac::Bbox::ThreeDimensional([
            0.0, 0.0, 0.0, 10.0, 10.0, 100.0,
        ])];
        collection
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

        // Read back with stac crate and verify
        let file = std::fs::File::open(&path).unwrap();
        let item_collection = stac::geoparquet::from_reader(file).unwrap();
        assert_eq!(item_collection.items.len(), 1);
        assert_eq!(item_collection.items[0].id, "item-1");
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
        let item_collection = stac::geoparquet::from_reader(file).unwrap();
        assert_eq!(item_collection.items.len(), 3);
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

        // Verify parquet metadata keys
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
        assert_eq!(geo_json["primary_column"], "geometry");

        let stac_kv = metadata
            .iter()
            .find(|kv| kv.key == "stac-geoparquet")
            .unwrap();
        let stac_json: serde_json::Value =
            serde_json::from_str(stac_kv.value.as_deref().unwrap()).unwrap();
        assert!(stac_json["collections"]["test-collection"].is_object());
    }

    #[test]
    fn test_roundtrip_properties() {
        let items = vec![make_test_item(
            "item-1",
            vec![4.0, 52.0, 0.0, 5.0, 53.0, 100.0],
        )];
        let collection = make_test_collection();

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("items.parquet");
        write_geoparquet(&items, &collection, &path).unwrap();

        // Read back and verify properties survive roundtrip
        let file = std::fs::File::open(&path).unwrap();
        let item_collection = stac::geoparquet::from_reader(file).unwrap();
        let item = &item_collection.items[0];

        assert_eq!(item.id, "item-1");
        assert!(item.bbox.is_some());
        assert!(item.geometry.is_some());
        assert!(item.properties.datetime.is_some());
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
