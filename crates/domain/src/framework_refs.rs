use serde::Deserialize;
use serde_json::Value;

/// One row in `data/framework_refs_seed.json` (merge into existing node refs).
#[derive(Debug, Clone, Deserialize)]
pub struct FrameworkRefsImportRow {
    pub skill_id: String,
    pub framework_refs: Value,
}

/// Deep-merge `patch` into `base` (object keys; patch wins on scalar conflict).
pub fn merge_framework_refs(base: &Value, patch: &Value) -> Value {
    match (base, patch) {
        (Value::Object(a), Value::Object(b)) => {
            let mut out = a.clone();
            for (k, v) in b {
                let merged = match out.get(k) {
                    Some(existing) => merge_framework_refs(existing, v),
                    None => v.clone(),
                };
                out.insert(k.clone(), merged);
            }
            Value::Object(out)
        }
        (_, patch) => patch.clone(),
    }
}

pub fn merge_framework_refs_into(base: &mut Value, patch: &Value) {
    *base = merge_framework_refs(base, patch);
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn merges_nested_objects() {
        let base = json!({ "nist": { "publication": "SP 800-53", "rev": "4" } });
        let patch = json!({ "nist": { "rev": "5", "family": "controls" } });
        let merged = merge_framework_refs(&base, &patch);
        assert_eq!(merged["nist"]["rev"], "5");
        assert_eq!(merged["nist"]["publication"], "SP 800-53");
        assert_eq!(merged["nist"]["family"], "controls");
    }
}
