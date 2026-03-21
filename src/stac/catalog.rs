//! STAC Catalog builder

pub struct StacCatalogBuilder {
    catalog: stac::Catalog,
}

impl StacCatalogBuilder {
    pub fn new(id: impl ToString, description: impl ToString) -> Self {
        Self {
            catalog: stac::Catalog::new(id, description),
        }
    }

    pub fn title(mut self, title: impl ToString) -> Self {
        self.catalog.title = Some(title.to_string());
        self
    }

    pub fn add_link(mut self, link: stac::Link) -> Self {
        self.catalog.links.push(link);
        self
    }

    pub fn child_link(mut self, href: impl ToString, title: Option<String>) -> Self {
        let mut link = stac::Link::child(href);
        link.title = title;
        self.catalog.links.push(link);
        self
    }

    pub fn self_link(mut self, href: impl ToString) -> Self {
        self.catalog.links.push(stac::Link::self_(href));
        self
    }

    pub fn build(self) -> stac::Catalog {
        self.catalog
    }
}
