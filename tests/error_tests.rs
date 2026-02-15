//! Unit tests for error handling

use city3d_stac::error::CityJsonStacError;
use std::io;

mod error_type_tests {
    use super::*;

    #[test]
    fn test_unsupported_format_error() {
        let err = CityJsonStacError::UnsupportedFormat(".xyz".to_string());
        let msg = err.to_string();
        assert!(msg.contains("Unsupported file format"));
        assert!(msg.contains(".xyz"));
    }

    #[test]
    fn test_io_error_from() {
        let io_err = io::Error::new(io::ErrorKind::NotFound, "file not found");
        let err = CityJsonStacError::from(io_err);
        let msg = err.to_string();
        assert!(msg.contains("IO error"));
    }

    #[test]
    fn test_json_error_from() {
        let json_str = "{ invalid json }";
        let json_err = serde_json::from_str::<serde_json::Value>(json_str).unwrap_err();
        let err = CityJsonStacError::from(json_err);
        let msg = err.to_string();
        assert!(msg.contains("JSON parsing error"));
    }

    #[test]
    fn test_metadata_error() {
        let err = CityJsonStacError::MetadataError("no bbox found".to_string());
        let msg = err.to_string();
        assert!(msg.contains("Failed to extract metadata"));
        assert!(msg.contains("no bbox found"));
    }

    #[test]
    fn test_invalid_cityjson_error() {
        let err = CityJsonStacError::InvalidCityJson("missing type field".to_string());
        let msg = err.to_string();
        assert!(msg.contains("Invalid CityJSON structure"));
        assert!(msg.contains("missing type field"));
    }

    #[test]
    fn test_missing_field_error() {
        let err = CityJsonStacError::MissingField("version".to_string());
        let msg = err.to_string();
        assert!(msg.contains("Missing required field"));
        assert!(msg.contains("version"));
    }

    #[test]
    fn test_stac_error() {
        let err = CityJsonStacError::StacError("invalid bbox".to_string());
        let msg = err.to_string();
        assert!(msg.contains("STAC generation error"));
        assert!(msg.contains("invalid bbox"));
    }

    #[test]
    fn test_geojson_error() {
        let err = CityJsonStacError::GeoJsonError("invalid coordinates".to_string());
        let msg = err.to_string();
        assert!(msg.contains("GeoJSON error"));
        assert!(msg.contains("invalid coordinates"));
    }

    #[test]
    fn test_no_files_found_error() {
        let err = CityJsonStacError::NoFilesFound;
        let msg = err.to_string();
        assert!(msg.contains("No supported files found"));
    }

    #[test]
    fn test_other_error() {
        let err = CityJsonStacError::Other("custom error message".to_string());
        let msg = err.to_string();
        assert!(msg.contains("custom error message"));
    }

    #[test]
    fn test_error_from_string() {
        let err: CityJsonStacError = "error from string".to_string().into();
        let msg = err.to_string();
        assert!(msg.contains("error from string"));
    }

    #[test]
    fn test_error_from_str() {
        let err: CityJsonStacError = "error from str".into();
        let msg = err.to_string();
        assert!(msg.contains("error from str"));
    }
}

mod error_display_tests {
    use super::*;

    #[test]
    fn test_error_is_debug() {
        let err = CityJsonStacError::NoFilesFound;
        let debug = format!("{:?}", err);
        assert!(debug.contains("NoFilesFound"));
    }

    #[test]
    fn test_error_is_display() {
        let err = CityJsonStacError::MissingField("test".to_string());
        let display = format!("{}", err);
        assert!(!display.is_empty());
    }

    #[test]
    fn test_unsupported_format_shows_supported_formats() {
        let err = CityJsonStacError::UnsupportedFormat(".foo".to_string());
        let msg = err.to_string();
        // Should mention supported formats in help message
        assert!(msg.contains(".json") || msg.contains("CityJSON"));
    }
}

mod result_type_tests {
    use city3d_stac::error::Result;

    #[test]
    fn test_result_ok() {
        let result: Result<i32> = Ok(42);
        assert!(result.is_ok());
        if let Ok(val) = result {
            assert_eq!(val, 42);
        }
    }

    #[test]
    fn test_result_err() {
        use super::*;
        let result: Result<i32> = Err(CityJsonStacError::NoFilesFound);
        assert!(result.is_err());
    }
}
