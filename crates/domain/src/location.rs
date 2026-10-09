use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct WorkLocation {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub city: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
}

impl WorkLocation {
    pub fn is_empty(&self) -> bool {
        self.city.as_ref().is_none_or(|s| s.trim().is_empty())
            && self.state.as_ref().is_none_or(|s| s.trim().is_empty())
    }
}
