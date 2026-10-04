use serde::{Deserialize, Serialize};
use serde_json::Value;

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

/// Split legacy `Company | City, ST` into company name + structured location.
pub fn split_legacy_company_field(company: &str) -> (String, Option<WorkLocation>) {
    let trimmed = company.trim();
    let Some((left, right)) = trimmed.split_once(" | ") else {
        return (trimmed.to_string(), None);
    };
    let name = left.trim().to_string();
    let location = parse_city_state(right.trim());
    (name, location.filter(|l| !l.is_empty()))
}

fn parse_city_state(s: &str) -> Option<WorkLocation> {
    if s.is_empty() {
        return None;
    }
    if let Some((city, state)) = s.split_once(',') {
        let city = city.trim();
        let state = state.trim();
        if city.is_empty() && state.is_empty() {
            return None;
        }
        return Some(WorkLocation {
            city: if city.is_empty() {
                None
            } else {
                Some(city.to_string())
            },
            state: if state.is_empty() {
                None
            } else {
                Some(state.to_string())
            },
        });
    }
    Some(WorkLocation {
        city: Some(s.to_string()),
        state: None,
    })
}

pub fn normalize_experience_location(payload: &mut Value) {
    let Some(obj) = payload.as_object_mut() else {
        return;
    };
    if obj.contains_key("location") {
        return;
    }
    let Some(Value::String(company)) = obj.get("company").cloned() else {
        return;
    };
    if !company.contains(" | ") {
        return;
    }
    let (name, location) = split_legacy_company_field(&company);
    obj.insert("company".to_string(), Value::String(name));
    if let Some(loc) = location {
        if let Ok(v) = serde_json::to_value(&loc) {
            obj.insert("location".to_string(), v);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_pipe_company_string() {
        let (name, loc) = split_legacy_company_field("Affinity DB | Leesburg, VA");
        assert_eq!(name, "Affinity DB");
        let loc = loc.unwrap();
        assert_eq!(loc.city.as_deref(), Some("Leesburg"));
        assert_eq!(loc.state.as_deref(), Some("VA"));
    }
}
