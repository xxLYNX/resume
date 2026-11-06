use serde_json::Value;

use crate::dates::{validate_date_range, DateRange};
use crate::location::WorkLocation;
use crate::ElementKind;

/// Long-form Markdown portfolio history for projects (not rendered on résumés by default).
pub const PROJECT_PORTFOLIO_NARRATIVE_KEY: &str = "portfolio_narrative";

#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("payload must be a JSON object")]
    NotObject,
    #[error("field `{0}` must be a string")]
    ExpectedString(&'static str),
    #[error("field `{0}` must be an array")]
    ExpectedArray(&'static str),
    #[error("field `{0}` must be an array of strings")]
    ExpectedStringArray(&'static str),
    #[error("field `{0}` must be an object")]
    ExpectedObject(&'static str),
    #[error("invalid date_range: {0}")]
    InvalidDateRange(#[from] crate::dates::DateError),
    #[error("field `date` is deprecated; use structured `date_range`")]
    DeprecatedDateField,
}

pub fn validate_payload(kind: ElementKind, payload: &Value) -> Result<(), ValidationError> {
    let obj = payload.as_object().ok_or(ValidationError::NotObject)?;

    match kind {
        ElementKind::Header => {
            check_optional_string(obj, "name")?;
            check_optional_string(obj, "phone")?;
            check_optional_string(obj, "email")?;
            check_optional_string(obj, "linkedin")?;
            check_optional_string(obj, "website")?;
            check_optional_string(obj, "github")?;
        }
        ElementKind::Objective => {
            check_optional_string(obj, "objective")?;
        }
        ElementKind::Education => {
            check_optional_string(obj, "degree")?;
            check_optional_string(obj, "school")?;
            check_optional_date_fields(obj)?;
            check_optional_string_string_array(obj, "highlights")?;
        }
        ElementKind::Experience => {
            check_optional_string(obj, "position")?;
            check_optional_string(obj, "company")?;
            check_optional_work_location(obj)?;
            check_optional_date_fields(obj)?;
            check_optional_string_string_array(obj, "highlights")?;
        }
        ElementKind::Project => {
            check_optional_string(obj, "name")?;
            check_optional_date_fields(obj)?;
            check_optional_string_string_array(obj, "details")?;
            check_optional_string(obj, PROJECT_PORTFOLIO_NARRATIVE_KEY)?;
        }
        ElementKind::Course => {
            check_optional_string(obj, "name")?;
            check_optional_string(obj, "code")?;
            check_optional_date_fields(obj)?;
        }
        ElementKind::Certification => {
            check_optional_string(obj, "name")?;
            check_optional_string(obj, "issuer")?;
            check_optional_date_fields(obj)?;
        }
        ElementKind::Award => {
            check_optional_string(obj, "title")?;
            check_optional_string(obj, "issuer")?;
            check_optional_date_fields(obj)?;
        }
    }

    Ok(())
}

fn check_optional_work_location(
    obj: &serde_json::Map<String, Value>,
) -> Result<(), ValidationError> {
    let Some(v) = obj.get("location") else {
        return Ok(());
    };
    if v.is_null() {
        return Ok(());
    }
    let _: WorkLocation = serde_json::from_value(v.clone())
        .map_err(|_| ValidationError::ExpectedObject("location"))?;
    Ok(())
}

fn check_optional_date_fields(obj: &serde_json::Map<String, Value>) -> Result<(), ValidationError> {
    if let Some(v) = obj.get("date_range") {
        if !v.is_null() {
            let range: DateRange = serde_json::from_value(v.clone())
                .map_err(|_| ValidationError::ExpectedObject("date_range"))?;
            validate_date_range(&range)?;
        }
    }
    if obj.contains_key("date") {
        return Err(ValidationError::DeprecatedDateField);
    }
    Ok(())
}

fn check_optional_string(
    obj: &serde_json::Map<String, Value>,
    key: &'static str,
) -> Result<(), ValidationError> {
    if let Some(v) = obj.get(key) {
        if !v.is_string() {
            return Err(ValidationError::ExpectedString(key));
        }
    }
    Ok(())
}

fn check_optional_string_string_array(
    obj: &serde_json::Map<String, Value>,
    key: &'static str,
) -> Result<(), ValidationError> {
    let Some(v) = obj.get(key) else {
        return Ok(());
    };
    let arr = v.as_array().ok_or(ValidationError::ExpectedArray(key))?;
    for item in arr {
        if !item.is_string() {
            return Err(ValidationError::ExpectedStringArray(key));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn project_payload_without_narrative_valid() {
        let payload = json!({
            "name": "Legacy",
            "details": ["Shipped feature"]
        });
        validate_payload(ElementKind::Project, &payload).unwrap();
    }

    #[test]
    fn project_portfolio_narrative_must_be_string() {
        let payload = json!({
            "name": "Bad",
            "portfolio_narrative": ["not", "a", "string"]
        });
        let err = validate_payload(ElementKind::Project, &payload).unwrap_err();
        assert!(matches!(err, ValidationError::ExpectedString(_)));
    }

    #[test]
    fn project_portfolio_narrative_accepts_long_markdown() {
        let narrative = "# History\n\n".repeat(200);
        let payload = json!({
            "name": "Rich",
            "portfolio_narrative": narrative
        });
        validate_payload(ElementKind::Project, &payload).unwrap();
    }
}
