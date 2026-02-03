//! 3D Bounding box implementation

use serde::{Deserialize, Serialize};

/// 3D Bounding box [xmin, ymin, zmin, xmax, ymax, zmax]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BBox3D {
    pub xmin: f64,
    pub ymin: f64,
    pub zmin: f64,
    pub xmax: f64,
    pub ymax: f64,
    pub zmax: f64,
}

impl BBox3D {
    /// Create a new 3D bounding box
    pub fn new(xmin: f64, ymin: f64, zmin: f64, xmax: f64, ymax: f64, zmax: f64) -> Self {
        Self {
            xmin,
            ymin,
            zmin,
            xmax,
            ymax,
            zmax,
        }
    }

    /// Convert to STAC bbox array format [xmin, ymin, zmin, xmax, ymax, zmax]
    pub fn to_array(&self) -> [f64; 6] {
        [
            self.xmin, self.ymin, self.zmin, self.xmax, self.ymax, self.zmax,
        ]
    }

    /// Merge two bounding boxes (union)
    pub fn merge(&self, other: &BBox3D) -> BBox3D {
        BBox3D {
            xmin: self.xmin.min(other.xmin),
            ymin: self.ymin.min(other.ymin),
            zmin: self.zmin.min(other.zmin),
            xmax: self.xmax.max(other.xmax),
            ymax: self.ymax.max(other.ymax),
            zmax: self.zmax.max(other.zmax),
        }
    }

    /// Get 2D footprint [xmin, ymin, xmax, ymax] (for STAC geometry)
    pub fn footprint_2d(&self) -> [f64; 4] {
        [self.xmin, self.ymin, self.xmax, self.ymax]
    }

    /// Check if the bounding box is valid (min values <= max values)
    pub fn is_valid(&self) -> bool {
        self.xmin <= self.xmax && self.ymin <= self.ymax && self.zmin <= self.zmax
    }

    /// Get the center point of the bounding box
    pub fn center(&self) -> (f64, f64, f64) {
        (
            (self.xmin + self.xmax) / 2.0,
            (self.ymin + self.ymax) / 2.0,
            (self.zmin + self.zmax) / 2.0,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bbox_creation() {
        let bbox = BBox3D::new(0.0, 0.0, 0.0, 10.0, 10.0, 10.0);
        assert_eq!(bbox.xmin, 0.0);
        assert_eq!(bbox.xmax, 10.0);
    }

    #[test]
    fn test_bbox_merge() {
        let bbox1 = BBox3D::new(0.0, 0.0, 0.0, 10.0, 10.0, 10.0);
        let bbox2 = BBox3D::new(5.0, 5.0, 5.0, 15.0, 15.0, 15.0);
        let merged = bbox1.merge(&bbox2);

        assert_eq!(merged.xmin, 0.0);
        assert_eq!(merged.ymin, 0.0);
        assert_eq!(merged.zmin, 0.0);
        assert_eq!(merged.xmax, 15.0);
        assert_eq!(merged.ymax, 15.0);
        assert_eq!(merged.zmax, 15.0);
    }

    #[test]
    fn test_bbox_to_array() {
        let bbox = BBox3D::new(1.0, 2.0, 3.0, 4.0, 5.0, 6.0);
        let array = bbox.to_array();
        assert_eq!(array, [1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
    }

    #[test]
    fn test_bbox_footprint_2d() {
        let bbox = BBox3D::new(1.0, 2.0, 3.0, 4.0, 5.0, 6.0);
        let footprint = bbox.footprint_2d();
        assert_eq!(footprint, [1.0, 2.0, 4.0, 5.0]);
    }

    #[test]
    fn test_bbox_is_valid() {
        let valid_bbox = BBox3D::new(0.0, 0.0, 0.0, 10.0, 10.0, 10.0);
        assert!(valid_bbox.is_valid());

        let invalid_bbox = BBox3D::new(10.0, 0.0, 0.0, 0.0, 10.0, 10.0);
        assert!(!invalid_bbox.is_valid());
    }

    #[test]
    fn test_bbox_center() {
        let bbox = BBox3D::new(0.0, 0.0, 0.0, 10.0, 10.0, 10.0);
        let center = bbox.center();
        assert_eq!(center, (5.0, 5.0, 5.0));
    }
}
