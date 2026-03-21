//! STAC generation module
//!
//! This module uses the upstream `stac` crate for core STAC types
//! (Item, Collection, Catalog, Asset, Link, etc.) and provides
//! builder patterns specific to 3D city model metadata.

mod accumulator;
mod catalog;
mod collection;
pub mod geoparquet;
mod item;
mod models;

pub use accumulator::{CollectionAccumulator, ItemMetadata};
pub use catalog::StacCatalogBuilder;
pub use collection::StacCollectionBuilder;
pub use item::StacItemBuilder;
pub use models::CityObjectsCount;

// Re-export upstream stac crate types used throughout the codebase.
// Type aliases preserve backward compatibility with existing code.
pub type StacItem = stac::Item;
pub type StacCollection = stac::Collection;
pub type StacCatalog = stac::Catalog;
pub type Asset = stac::Asset;
pub type Link = stac::Link;
pub type Provider = stac::Provider;
pub type Extent = stac::Extent;
pub type SpatialExtent = stac::SpatialExtent;
pub type TemporalExtent = stac::TemporalExtent;
pub type Bbox = stac::Bbox;
pub type ItemAsset = stac::ItemAsset;
