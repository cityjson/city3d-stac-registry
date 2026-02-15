//! # CityJSON-STAC
//!
//! A library for generating STAC (SpatioTemporal Asset Catalog) metadata
//! from CityJSON datasets.
//!
//! ## Supported Formats
//!
//! - CityJSON (`.json`)
//! - CityJSON Sequences (`.jsonl`)
//! - FlatCityBuf (`.fcb`) - coming soon
//!
//! ## Example
//!
//! ```no_run
//! use city3d_stac::reader::get_reader;
//! use city3d_stac::stac::StacItemBuilder;
//! use std::path::Path;
//!
//! # fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let reader = get_reader(Path::new("building.json"))?;
//! let item = StacItemBuilder::new("my-building")
//!     .bbox(reader.bbox()?)
//!     .cityjson_metadata(&*reader)?
//!     .build()?;
//! # Ok(())
//! # }
//! ```

pub mod cli;
pub mod config;
pub mod error;
pub mod metadata;
pub mod reader;
pub mod remote;
pub mod stac;
pub mod traversal;

pub use error::{CityJsonStacError, Result};
