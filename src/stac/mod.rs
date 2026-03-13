//! STAC generation module
//!
//! This module exports STAC types derived from STAC v1.0.0 JSON schemas.
//!
//! The types match the official STAC specification structure with serde
//! annotations for proper JSON serialization/deserialization.
//!
//! ## Type Generation
//!
//! STAC types are manually maintained in `models.rs` and are derived from
//! the official STAC v1.0.0 JSON schemas located in `stac-spec/`:
//! - item-spec/json-schema/item.json
//! - collection-spec/json-schema/collection.json
//!
//! To modify STAC types, edit `src/stac/models.rs` and run `cargo build`.

mod accumulator;
mod catalog;
mod collection;
pub mod geoparquet;
mod item;
mod models;

pub use accumulator::{CollectionAccumulator, ItemMetadata};
pub use catalog::{StacCatalog, StacCatalogBuilder};
pub use collection::StacCollectionBuilder;
pub use item::StacItemBuilder;
pub use models::{
    Asset, CityObjectsCount, Extent, Link, Provider, SpatialExtent, StacCollection, StacItem,
    TemporalExtent,
};
