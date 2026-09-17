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
