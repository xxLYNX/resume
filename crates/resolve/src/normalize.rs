use domain::enrich_payload_dates;
use serde_json::{json, Map, Value};

/// Ensure list-section rows have safe defaults before templates run.
pub fn normalize_section_value(section_key: &str, ctx: Value) -> Value {
    match section_key {
        "education" => normalize_list_section(ctx, "education", &["highlights"]),
        "experience" => normalize_list_section(ctx, "experience", &["highlights"]),
        "projects" => normalize_list_section(ctx, "projects", &["details"]),
        _ => ctx,
    }
}

fn normalize_list_section(ctx: Value, list_key: &str, array_fields: &[&str]) -> Value {
    let Some(root) = ctx.as_object() else {
        return ctx;
    };
    let Some(items) = root.get(list_key).and_then(|v| v.as_array()) else {
        return ctx;
    };

    let mut normalized_items = Vec::with_capacity(items.len());
    for item in items {
        normalized_items.push(normalize_row(item, array_fields));
    }

    let mut out = root.clone();
    out.insert(list_key.to_string(), Value::Array(normalized_items));
    Value::Object(out)
}

fn normalize_row(item: &Value, array_fields: &[&str]) -> Value {
    let Some(obj) = item.as_object() else {
        return item.clone();
    };
    let mut out: Map<String, Value> = obj.clone();
    for key in array_fields {
        match out.get(*key) {
            None | Some(Value::Null) => {
                out.insert(key.to_string(), json!([]));
            }
            Some(Value::Array(_)) => {}
            _ => {}
        }
    }
    // Empty optional scalars are omitted so theme `{% if row.field %}` works.
    for key in ["technologies", "gpa"] {
        if matches!(out.get(key), Some(Value::String(s)) if s.is_empty()) {
            out.remove(key);
        }
    }
    let mut row = Value::Object(out);
    enrich_payload_dates(&mut row);
    row
}
