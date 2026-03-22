//! Coordinate Reference System information

use serde::{Deserialize, Serialize};

/// Coordinate Reference System information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CRS {
    /// EPSG code (e.g., 7415 for EPSG:7415)
    pub epsg: Option<u32>,

    /// WKT2 representation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wkt2: Option<String>,

    /// PROJ.4 string
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proj4: Option<String>,

    /// CityJSON authority name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authority: Option<String>,

    /// CityJSON identifier (usually the EPSG code as string)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identifier: Option<String>,
}

impl CRS {
    /// Create a CRS from an EPSG code
    pub fn from_epsg(code: u32) -> Self {
        Self {
            epsg: Some(code),
            wkt2: None,
            proj4: None,
            authority: Some("EPSG".to_string()),
            identifier: Some(code.to_string()),
        }
    }

    /// Create an unknown CRS (no coordinate reference system information available)
    pub fn unknown() -> Self {
        Self {
            epsg: None,
            wkt2: None,
            proj4: None,
            authority: None,
            identifier: None,
        }
    }

    /// Returns true if this CRS has a known EPSG code
    pub fn is_known(&self) -> bool {
        self.epsg.is_some()
    }

    /// Create a CRS from CityJSON metadata format
    /// CityJSON stores CRS as a URL like: "https://www.opengis.net/def/crs/EPSG/0/7415"
    pub fn from_cityjson_url(url: &str) -> Option<Self> {
        // Parse EPSG code from URL (use next_back for efficiency on DoubleEndedIterator)
        if let Some(parts) = url.split('/').next_back() {
            if let Ok(code) = parts.parse::<u32>() {
                return Some(Self::from_epsg(code));
            }
        }
        None
    }

    /// Create a CRS from CityGML srsName format
    ///
    /// Supports multiple formats:
    /// - OGC URN (comma): `urn:ogc:def:crs,crs:EPSG::31256`
    /// - OGC URN (colon): `urn:ogc:def:crs:EPSG::31256`
    /// - Simple format: `EPSG:31256`
    /// - OGC URL: `http://www.opengis.net/def/crs/EPSG/0/31256`
    pub fn from_citygml_srs_name(srs_name: &str) -> Option<Self> {
        let srs_name = srs_name.trim();

        // Format 1: OGC URN with comma separator
        // urn:ogc:def:crs,crs:EPSG::31256
        if srs_name.starts_with("urn:ogc:def:crs") {
            // Try to find EPSG:: followed by a number
            if let Some(epsg_part) = srs_name.split("EPSG::").nth(1) {
                if let Ok(code) = epsg_part.split(&[',', ':', '/'][..]).next()?.parse::<u32>() {
                    return Some(Self::from_epsg(code));
                }
            }
            // Also try EPSG: without double colon (some variants)
            if let Some(epsg_part) = srs_name.split("EPSG:").nth(1) {
                // Skip if it's already EPSG:: (handled above)
                if !epsg_part.starts_with(':') {
                    if let Ok(code) = epsg_part.split(&[',', ':', '/'][..]).next()?.parse::<u32>() {
                        return Some(Self::from_epsg(code));
                    }
                }
            }
        }

        // Format 2: Simple EPSG:XXXX format
        // EPSG:31256
        if srs_name.starts_with("EPSG:") {
            if let Ok(code) = srs_name.strip_prefix("EPSG:")?.parse::<u32>() {
                return Some(Self::from_epsg(code));
            }
        }

        // Format 3: OGC HTTP URL
        // http://www.opengis.net/def/crs/EPSG/0/31256
        if srs_name.contains("opengis.net/def/crs") || srs_name.contains("opengis.net/gml/srs") {
            return Self::from_cityjson_url(srs_name);
        }

        None
    }

    /// Get EPSG code for STAC proj:epsg property (legacy)
    pub fn to_stac_epsg(&self) -> Option<u32> {
        self.epsg
    }

    /// Get proj:code string for STAC Projection Extension v2.0.0+
    ///
    /// Returns a string like "EPSG:7415" suitable for the `proj:code` property.
    pub fn to_stac_proj_code(&self) -> Option<String> {
        self.epsg.map(|code| format!("EPSG:{code}"))
    }

    /// Get the CRS as a CityJSON-compatible URL
    pub fn to_cityjson_url(&self) -> Option<String> {
        self.epsg
            .map(|code| format!("https://www.opengis.net/def/crs/EPSG/0/{code}"))
    }

    /// WGS84 CRS (EPSG:4326)
    pub fn wgs84() -> Self {
        Self::from_epsg(4326)
    }

    /// Returns true if this CRS uses WGS84 geographic coordinates.
    ///
    /// Matches EPSG:4326 (2D), EPSG:4979 (3D with ellipsoidal height),
    /// and EPSG:4978 (geocentric XYZ). All share the WGS84 datum with
    /// longitude/latitude axes, so no horizontal reprojection is needed.
    pub fn is_wgs84(&self) -> bool {
        matches!(self.epsg, Some(4326) | Some(4979) | Some(4978))
    }
}

impl Default for CRS {
    /// Default to unknown CRS (no EPSG code).
    /// Use `CRS::wgs84()` explicitly when WGS84 is intended.
    fn default() -> Self {
        Self::unknown()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crs_from_epsg() {
        let crs = CRS::from_epsg(7415);
        assert_eq!(crs.epsg, Some(7415));
        assert_eq!(crs.authority, Some("EPSG".to_string()));
        assert_eq!(crs.identifier, Some("7415".to_string()));
    }

    #[test]
    fn test_crs_from_cityjson_url() {
        let url = "https://www.opengis.net/def/crs/EPSG/0/7415";
        let crs = CRS::from_cityjson_url(url).unwrap();
        assert_eq!(crs.epsg, Some(7415));
    }

    #[test]
    fn test_crs_to_stac_epsg() {
        let crs = CRS::from_epsg(7415);
        assert_eq!(crs.to_stac_epsg(), Some(7415));
    }

    #[test]
    fn test_crs_to_stac_proj_code() {
        let crs = CRS::from_epsg(7415);
        assert_eq!(crs.to_stac_proj_code(), Some("EPSG:7415".to_string()));
    }

    #[test]
    fn test_crs_to_stac_proj_code_unknown() {
        let crs = CRS::unknown();
        assert_eq!(crs.to_stac_proj_code(), None);
    }

    #[test]
    fn test_crs_to_cityjson_url() {
        let crs = CRS::from_epsg(7415);
        assert_eq!(
            crs.to_cityjson_url(),
            Some("https://www.opengis.net/def/crs/EPSG/0/7415".to_string())
        );
    }

    #[test]
    fn test_crs_default() {
        // Default is now unknown CRS (not WGS84)
        let crs = CRS::default();
        assert_eq!(crs.epsg, None);
        assert!(!crs.is_known());
    }

    #[test]
    fn test_crs_unknown() {
        let crs = CRS::unknown();
        assert_eq!(crs.epsg, None);
        assert!(!crs.is_known());
    }

    #[test]
    fn test_crs_wgs84() {
        let crs = CRS::wgs84();
        assert_eq!(crs.epsg, Some(4326));
        assert!(crs.is_known());
        assert!(crs.is_wgs84());
    }

    #[test]
    fn test_crs_is_wgs84_false_for_other_epsg() {
        let crs = CRS::from_epsg(28992);
        assert!(!crs.is_wgs84());
    }

    #[test]
    fn test_crs_from_citygml_srs_name_urn_comma() {
        // Vienna CityGML format
        let srs = "urn:ogc:def:crs,crs:EPSG::31256";
        let crs = CRS::from_citygml_srs_name(srs).unwrap();
        assert_eq!(crs.epsg, Some(31256));
    }

    #[test]
    fn test_crs_from_citygml_srs_name_urn_colon() {
        let srs = "urn:ogc:def:crs:EPSG::4326";
        let crs = CRS::from_citygml_srs_name(srs).unwrap();
        assert_eq!(crs.epsg, Some(4326));
    }

    #[test]
    fn test_crs_from_citygml_srs_name_simple() {
        let srs = "EPSG:25832";
        let crs = CRS::from_citygml_srs_name(srs).unwrap();
        assert_eq!(crs.epsg, Some(25832));
    }

    #[test]
    fn test_crs_from_citygml_srs_name_http_url() {
        let srs = "http://www.opengis.net/def/crs/EPSG/0/7415";
        let crs = CRS::from_citygml_srs_name(srs).unwrap();
        assert_eq!(crs.epsg, Some(7415));
    }

    #[test]
    fn test_crs_from_citygml_srs_name_with_whitespace() {
        let srs = "  urn:ogc:def:crs,crs:EPSG::31256  ";
        let crs = CRS::from_citygml_srs_name(srs).unwrap();
        assert_eq!(crs.epsg, Some(31256));
    }

    #[test]
    fn test_crs_from_citygml_srs_name_invalid() {
        assert!(CRS::from_citygml_srs_name("INVALID").is_none());
        assert!(CRS::from_citygml_srs_name("").is_none());
    }
}
