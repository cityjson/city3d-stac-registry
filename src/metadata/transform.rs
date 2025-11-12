//! Coordinate transform parameters for vertex compression

use serde::{Deserialize, Serialize};

/// Coordinate transform parameters used for vertex compression in CityJSON
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Transform {
    /// Scale factors [x, y, z]
    pub scale: [f64; 3],

    /// Translation offsets [x, y, z]
    pub translate: [f64; 3],
}

impl Transform {
    /// Create a new transform
    pub fn new(scale: [f64; 3], translate: [f64; 3]) -> Self {
        Self { scale, translate }
    }

    /// Apply transform to compressed integer coordinates
    ///
    /// Formula: real_coord = compressed_coord * scale + translate
    pub fn apply(&self, compressed: &[i32; 3]) -> [f64; 3] {
        [
            compressed[0] as f64 * self.scale[0] + self.translate[0],
            compressed[1] as f64 * self.scale[1] + self.translate[1],
            compressed[2] as f64 * self.scale[2] + self.translate[2],
        ]
    }

    /// Apply transform to a batch of compressed coordinates
    pub fn apply_batch(&self, compressed: &[[i32; 3]]) -> Vec<[f64; 3]> {
        compressed.iter().map(|c| self.apply(c)).collect()
    }

    /// Inverse transform: convert real coordinates to compressed integers
    ///
    /// Formula: compressed_coord = round((real_coord - translate) / scale)
    pub fn inverse(&self, real: &[f64; 3]) -> [i32; 3] {
        [
            ((real[0] - self.translate[0]) / self.scale[0]).round() as i32,
            ((real[1] - self.translate[1]) / self.scale[1]).round() as i32,
            ((real[2] - self.translate[2]) / self.scale[2]).round() as i32,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transform_creation() {
        let transform = Transform::new(
            [0.001, 0.001, 0.001],
            [4629170.0, 5804690.0, 0.0],
        );
        assert_eq!(transform.scale, [0.001, 0.001, 0.001]);
        assert_eq!(transform.translate, [4629170.0, 5804690.0, 0.0]);
    }

    #[test]
    fn test_transform_apply() {
        let transform = Transform::new(
            [0.001, 0.001, 0.001],
            [1000.0, 2000.0, 0.0],
        );

        let compressed = [100, 200, 300];
        let real = transform.apply(&compressed);

        assert_eq!(real[0], 1000.1);
        assert_eq!(real[1], 2000.2);
        assert_eq!(real[2], 0.3);
    }

    #[test]
    fn test_transform_apply_batch() {
        let transform = Transform::new(
            [0.001, 0.001, 0.001],
            [1000.0, 2000.0, 0.0],
        );

        let compressed = vec![
            [100, 200, 300],
            [150, 250, 350],
        ];

        let real = transform.apply_batch(&compressed);
        assert_eq!(real.len(), 2);
        assert_eq!(real[0][0], 1000.1);
        assert_eq!(real[1][0], 1000.15);
    }

    #[test]
    fn test_transform_inverse() {
        let transform = Transform::new(
            [0.001, 0.001, 0.001],
            [1000.0, 2000.0, 0.0],
        );

        let real = [1000.1, 2000.2, 0.3];
        let compressed = transform.inverse(&real);

        assert_eq!(compressed, [100, 200, 300]);
    }

    #[test]
    fn test_transform_roundtrip() {
        let transform = Transform::new(
            [0.001, 0.001, 0.001],
            [1000.0, 2000.0, 0.0],
        );

        let original = [100, 200, 300];
        let real = transform.apply(&original);
        let compressed = transform.inverse(&real);

        assert_eq!(compressed, original);
    }

    #[test]
    fn test_transform_serialization() {
        let transform = Transform::new(
            [0.001, 0.001, 0.001],
            [1000.0, 2000.0, 0.0],
        );

        let json = serde_json::to_string(&transform).unwrap();
        let deserialized: Transform = serde_json::from_str(&json).unwrap();

        assert_eq!(transform, deserialized);
    }
}
