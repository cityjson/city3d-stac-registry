//! Domain-specific STAC types for 3D City Models
//!
//! This module contains types specific to the city3d STAC extension
//! that are not part of the upstream `stac` crate.

use serde::{Deserialize, Serialize};

/// City object count - either integer or statistics object
///
/// For STAC Items, this is typically a single integer.
/// For STAC Collections, this can be statistics with min/max/total.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CityObjectsCount {
    Integer(u64),
    Statistics { min: u64, max: u64, total: u64 },
}

impl From<u64> for CityObjectsCount {
    fn from(value: u64) -> Self {
        CityObjectsCount::Integer(value)
    }
}

impl From<(u64, u64, u64)> for CityObjectsCount {
    fn from((min, max, total): (u64, u64, u64)) -> Self {
        CityObjectsCount::Statistics { min, max, total }
    }
}
