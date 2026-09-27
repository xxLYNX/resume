use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Composition {
    pub sections: Vec<CompositionSection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompositionSection {
    pub section_key: String,
    pub items: Vec<CompositionItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompositionItem {
    pub element_id: String,
    pub revision_id: String,
}

impl Composition {
    pub fn references_element(&self, element_id: &str) -> bool {
        self.sections.iter().any(|section| {
            section
                .items
                .iter()
                .any(|item| item.element_id == element_id)
        })
    }
}
