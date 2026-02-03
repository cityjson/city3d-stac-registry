//! Metadata structures for CityJSON datasets

mod attributes;
mod bbox;
mod crs;
mod transform;

pub use attributes::{AttributeDefinition, AttributeType};
pub use bbox::BBox3D;
pub use crs::CRS;
pub use transform::Transform;
