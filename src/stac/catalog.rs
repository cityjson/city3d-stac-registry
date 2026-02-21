//! STAC Catalog implementation

use crate::stac::Link;
use serde::{Deserialize, Serialize};

/// STAC Catalog
///
/// Corresponds to the STAC Catalog specification:
/// https://github.com/radiantearth/stac-spec/blob/master/catalog-spec/catalog-spec.md
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StacCatalog {
    #[serde(rename = "stac_version")]
    pub stac_version: String,

    #[serde(rename = "stac_extensions")]
    pub stac_extensions: Vec<String>,

    #[serde(rename = "type")]
    pub catalog_type: String,

    pub id: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    pub description: String,

    pub links: Vec<Link>,
}

impl StacCatalog {
    pub fn new(id: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            stac_version: "1.1.0".to_string(),
            stac_extensions: Vec::new(),
            catalog_type: "Catalog".to_string(),
            id: id.into(),
            title: None,
            description: description.into(),
            links: Vec::new(),
        }
    }
}

pub struct StacCatalogBuilder {
    catalog: StacCatalog,
}

impl StacCatalogBuilder {
    pub fn new(id: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            catalog: StacCatalog::new(id, description),
        }
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.catalog.title = Some(title.into());
        self
    }

    pub fn add_link(mut self, link: Link) -> Self {
        self.catalog.links.push(link);
        self
    }

    pub fn child_link(mut self, href: impl Into<String>, title: Option<String>) -> Self {
        let mut link = Link::new("child", href).with_type("application/json");
        if let Some(t) = title {
            link = link.with_title(t);
        }
        self.catalog.links.push(link);
        self
    }

    pub fn self_link(mut self, href: impl Into<String>) -> Self {
        self.catalog
            .links
            .push(Link::new("self", href).with_type("application/json"));
        self
    }

    pub fn build(self) -> StacCatalog {
        self.catalog
    }
}
