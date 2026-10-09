use serde_json::Value;

use crate::dates::{validate_date_range, DateRange};
use crate::location::WorkLocation;
use crate::ElementKind;

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
    #[error("skills section requires a `skills` object")]
    MissingSkillsObject,
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
            check_optional_string(obj, "technologies")?;
            check_optional_date_fields(obj)?;
            check_optional_string_string_array(obj, "details")?;
        }
        ElementKind::Skills => {
            if !obj.contains_key("skills") {
                return Err(ValidationError::MissingSkillsObject);
            }
            let skills = obj
                .get("skills")
                .ok_or(ValidationError::MissingSkillsObject)?;
            let skills_obj = skills.as_object().ok_or(ValidationError::ExpectedObject("skills"))?;
            check_optional_string(skills_obj, "certifications")?;
            check_optional_string(skills_obj, "languages")?;
            check_optional_string(skills_obj, "securityTools")?;
            check_optional_string(skills_obj, "other")?;
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
    let _: WorkLocation =
        serde_json::from_value(v.clone()).map_err(|_| ValidationError::ExpectedObject("location"))?;
    Ok(())
}

fn check_optional_date_fields(
    obj: &serde_json::Map<String, Value>,
) -> Result<(), ValidationError> {
    if let Some(v) = obj.get("date_range") {
        if !v.is_null() {
            let range: DateRange =
                serde_json::from_value(v.clone()).map_err(|_| {
                    ValidationError::ExpectedObject("date_range")
                })?;
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
