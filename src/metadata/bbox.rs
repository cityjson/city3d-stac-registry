//! 3D Bounding box implementation

use crate::error::{CityJsonStacError, Result};
use crate::metadata::crs::CRS;
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

    /// Transform this bounding box from its source CRS to WGS84 (EPSG:4326)
    ///
    /// STAC requires bbox and geometry to be in WGS84 (per GeoJSON RFC 7946).
    /// This method reprojects the 2D footprint corners and computes a new
    /// enclosing bbox in WGS84. Z values are preserved as-is.
    ///
    /// If the source CRS is already WGS84, returns a clone unchanged.
    /// If the source CRS is unknown (no EPSG code), checks whether the
    /// coordinates are within WGS84 range. If they are not, returns an error
    /// because the coordinates cannot be safely assumed to be in WGS84.
    pub fn to_wgs84(&self, source_crs: &CRS) -> Result<BBox3D> {
        let epsg = match source_crs.epsg {
            Some(code) => code,
            None => {
                // No CRS info: validate that bbox is within WGS84 valid range.
                // WGS84 longitude: [-180, 180], latitude: [-90, 90].
                let x_in_range = self.xmin >= -180.0 && self.xmax <= 180.0;
                let y_in_range = self.ymin >= -90.0 && self.ymax <= 90.0;
                if x_in_range && y_in_range {
                    // Coordinates look like WGS84 — return unchanged.
                    return Ok(self.clone());
                }
                // Coordinates are outside WGS84 range but CRS is unknown.
                return Err(CityJsonStacError::Other(format!(
                    "Bounding box [{}, {}, {}, {}] is outside WGS84 range but no CRS is \
                     specified in the file. STAC requires WGS84 coordinates. Please add a \
                     'referenceSystem' to the CityJSON metadata so the coordinates can be \
                     reprojected correctly.",
                    self.xmin, self.ymin, self.xmax, self.ymax
                )));
            }
        };

        // Already WGS84 (2D or 3D)
        if matches!(epsg, 4326 | 4979) {
            return Ok(self.clone());
        }

        // Resolve the horizontal EPSG code for compound CRS
        // (e.g., EPSG:7415 = EPSG:28992 + EPSG:5709)
        let horizontal_epsg = resolve_horizontal_epsg(epsg);

        // Geographic CRS that are essentially identical to WGS84.
        // CityGML files with geographic CRS use (lat, lon) axis order per EPSG,
        // but STAC/GeoJSON requires (lon, lat). We detect (lat, lon) order by
        // checking if x values are in latitude range [-90, 90] while y values
        // are in longitude range, then swap. JGD2011 (EPSG:6668) is based on
        // GRS80 which differs from WGS84 by < 1mm, so no reprojection is needed.
        if is_wgs84_equivalent_geographic(horizontal_epsg) {
            if self.xmin >= -90.0 && self.xmax <= 90.0 {
                // Coordinates appear to be in (lat, lon) order — swap to (lon, lat)
                return Ok(BBox3D::new(
                    self.ymin, self.xmin, self.zmin, self.ymax, self.xmax, self.zmax,
                ));
            }
            // Already in (lon, lat) order (e.g., from CityJSON)
            return Ok(self.clone());
        }

        // epsg code is u32 but proj4rs uses u16
        let proj_code = u16::try_from(horizontal_epsg).map_err(|_| {
            CityJsonStacError::Other(format!("EPSG code {horizontal_epsg} exceeds u16 range"))
        })?;

        let src_proj = proj4rs::Proj::from_epsg_code(proj_code).map_err(|e| {
            CityJsonStacError::Other(format!(
                "Failed to create projection for EPSG:{horizontal_epsg}: {e}"
            ))
        })?;
        let dst_proj = proj4rs::Proj::from_proj_string("+proj=longlat +datum=WGS84 +no_defs")
            .map_err(|e| {
                CityJsonStacError::Other(format!("Failed to create WGS84 projection: {e}"))
            })?;

        // Transform the 4 corners of the 2D bounding box
        let corners: [(f64, f64); 4] = [
            (self.xmin, self.ymin),
            (self.xmax, self.ymin),
            (self.xmax, self.ymax),
            (self.xmin, self.ymax),
        ];

        let mut lons = Vec::with_capacity(4);
        let mut lats = Vec::with_capacity(4);

        for (x, y) in &corners {
            let mut point = (*x, *y, 0.0_f64);
            proj4rs::transform::transform(&src_proj, &dst_proj, &mut point).map_err(|e| {
                CityJsonStacError::Other(format!(
                    "Failed to transform coordinates ({x}, {y}) from EPSG:{horizontal_epsg} to WGS84: {e}"
                ))
            })?;
            // proj4rs outputs radians for longlat projections
            lons.push(point.0.to_degrees());
            lats.push(point.1.to_degrees());
        }

        let lon_min = lons.iter().cloned().fold(f64::INFINITY, f64::min);
        let lon_max = lons.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let lat_min = lats.iter().cloned().fold(f64::INFINITY, f64::min);
        let lat_max = lats.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

        // STAC bbox: [west, south, zmin, east, north, zmax]
        // In WGS84: x = longitude, y = latitude
        Ok(BBox3D::new(
            lon_min, lat_min, self.zmin, lon_max, lat_max, self.zmax,
        ))
    }
}

/// Check if an EPSG code is a geographic CRS essentially identical to WGS84.
/// These CRS are based on datums (like GRS80) that differ from WGS84 by < 1mm,
/// so no reprojection is needed — only axis order normalization.
fn is_wgs84_equivalent_geographic(epsg: u32) -> bool {
    matches!(
        epsg,
        4326  // WGS84 geographic 2D
        | 4979 // WGS84 geographic 3D (with ellipsoidal height)
        | 6668 // JGD2011 geographic 2D (GRS80 ellipsoid)
    )
}

/// Resolve compound CRS to their horizontal component EPSG code
///
/// Compound CRS combine a horizontal and vertical CRS (e.g., EPSG:7415 = EPSG:28992 + EPSG:5709).
/// For horizontal reprojection to WGS84, we only need the horizontal component.
fn resolve_horizontal_epsg(epsg: u32) -> u32 {
    match epsg {
        // Netherlands: Amersfoort / RD New + NAP height
        7415 => 28992,
        // Germany: ETRS89 / UTM zone 32N + DHHN2016 height
        9518 => 25832,
        // Germany: ETRS89 / UTM zone 33N + DHHN2016 height
        9519 => 25833,
        // Switzerland: CH1903+ / LV95 + LN02 height
        6150 => 2056,
        // Austria: MGI / Austria Lambert + Austrian vertical ref
        5775 => 31287,
        // Belgium: Belge 1972 / Belgian Lambert 72 + Ostend height
        6190 => 31370,
        // Japan: JGD2011 (geographic 3D) → JGD2011 (geographic 2D)
        6697 => 6668,
        // Not a known compound CRS, use as-is
        _ => epsg,
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

    #[test]
    fn test_to_wgs84_already_wgs84() {
        let bbox = BBox3D::new(4.0, 52.0, 0.0, 5.0, 53.0, 30.0);
        let crs = CRS::from_epsg(4326);
        let result = bbox.to_wgs84(&crs).unwrap();
        assert_eq!(result, bbox);
    }

    #[test]
    fn test_to_wgs84_no_crs_wgs84_range() {
        // Coordinates in WGS84 range with unknown CRS → returned unchanged
        let bbox = BBox3D::new(4.0, 52.0, 0.0, 5.0, 53.0, 30.0);
        let crs = CRS::unknown();
        let result = bbox.to_wgs84(&crs).unwrap();
        assert_eq!(result, bbox);
    }

    #[test]
    fn test_to_wgs84_no_crs_out_of_range() {
        // Coordinates outside WGS84 range with unknown CRS → should return an error
        // (e.g. Vienna in Austrian projected coordinates)
        let bbox = BBox3D::new(983.16, 340433.878, 27.861, 1510.432, 341048.5, 84.987);
        let crs = CRS::unknown();
        let result = bbox.to_wgs84(&crs);
        assert!(
            result.is_err(),
            "Expected error for out-of-WGS84-range bbox with unknown CRS"
        );
    }

    #[test]
    fn test_to_wgs84_from_rd_new() {
        // Bbox around Delft in EPSG:28992 (Amersfoort / RD New)
        let bbox = BBox3D::new(84000.0, 446000.0, 0.0, 85000.0, 447000.0, 30.0);
        let crs = CRS::from_epsg(28992);
        let result = bbox.to_wgs84(&crs).unwrap();

        // Delft is approximately at lon 4.35, lat 52.0
        assert!(
            result.xmin > 4.0 && result.xmin < 5.0,
            "lon_min should be ~4.3, got {}",
            result.xmin
        );
        assert!(
            result.ymin > 51.5 && result.ymin < 52.5,
            "lat_min should be ~52.0, got {}",
            result.ymin
        );
        assert!(
            result.xmax > 4.0 && result.xmax < 5.0,
            "lon_max should be ~4.35, got {}",
            result.xmax
        );
        assert!(
            result.ymax > 51.5 && result.ymax < 52.5,
            "lat_max should be ~52.01, got {}",
            result.ymax
        );

        // Z values should be preserved
        assert_eq!(result.zmin, 0.0);
        assert_eq!(result.zmax, 30.0);

        // Bbox should be valid (min <= max)
        assert!(result.is_valid());
    }

    #[test]
    fn test_to_wgs84_from_compound_crs_7415() {
        // EPSG:7415 = EPSG:28992 + EPSG:5709 (NAP height)
        // Should resolve to horizontal component 28992
        let bbox = BBox3D::new(84000.0, 446000.0, 0.0, 85000.0, 447000.0, 30.0);
        let crs = CRS::from_epsg(7415);
        let result = bbox.to_wgs84(&crs).unwrap();

        // Should produce same result as EPSG:28992
        let crs_28992 = CRS::from_epsg(28992);
        let result_28992 = bbox.to_wgs84(&crs_28992).unwrap();

        assert!((result.xmin - result_28992.xmin).abs() < 1e-10);
        assert!((result.ymin - result_28992.ymin).abs() < 1e-10);
        assert!((result.xmax - result_28992.xmax).abs() < 1e-10);
        assert!((result.ymax - result_28992.ymax).abs() < 1e-10);
    }

    #[test]
    fn test_to_wgs84_from_utm() {
        // Bbox in EPSG:25832 (ETRS89 / UTM zone 32N)
        let bbox = BBox3D::new(500000.0, 5700000.0, 0.0, 501000.0, 5701000.0, 50.0);
        let crs = CRS::from_epsg(25832);
        let result = bbox.to_wgs84(&crs).unwrap();

        // UTM zone 32N central meridian is 9°E, this point should be around 9°E, ~51.4°N
        assert!(
            result.xmin > 8.0 && result.xmin < 10.0,
            "lon should be ~9, got {}",
            result.xmin
        );
        assert!(
            result.ymin > 51.0 && result.ymin < 52.0,
            "lat should be ~51.4, got {}",
            result.ymin
        );
        assert!(result.is_valid());
    }

    #[test]
    fn test_resolve_horizontal_epsg() {
        assert_eq!(resolve_horizontal_epsg(7415), 28992);
        assert_eq!(resolve_horizontal_epsg(9518), 25832);
        assert_eq!(resolve_horizontal_epsg(6697), 6668); // JGD2011 3D → 2D
        assert_eq!(resolve_horizontal_epsg(28992), 28992); // Not compound, returned as-is
        assert_eq!(resolve_horizontal_epsg(4326), 4326); // WGS84, returned as-is
    }

    #[test]
    fn test_to_wgs84_from_jgd2011_3d() {
        // EPSG:6697 = JGD2011 geographic 3D (lat/lon + height)
        // Sendai area coordinates from PLATEAU dataset
        let bbox = BBox3D::new(38.275, 141.037, 0.0, 38.283, 141.043, 54.7);
        let crs = CRS::from_epsg(6697);
        let result = bbox.to_wgs84(&crs).unwrap();
        // JGD2011 is essentially the same as WGS84 (sub-meter difference)
        // so the output should be very close to the input
        assert!((result.xmin - 141.037).abs() < 0.001);
        assert!((result.ymin - 38.275).abs() < 0.001);
        assert!((result.xmax - 141.043).abs() < 0.001);
        assert!((result.ymax - 38.283).abs() < 0.001);
    }
}
