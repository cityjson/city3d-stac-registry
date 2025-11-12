//! Metadata structures for CityJSON datasets

mod bbox;
mod crs;
mod attributes;
mod transform;

pub use bbox::BBox3D;
pub use crs::CRS;
pub use attributes::{AttributeDefinition, AttributeType};
pub use transform::Transform;
