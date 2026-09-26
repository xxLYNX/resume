use serde::{Deserialize, Serialize};

pub const EMPLOYMENT_TYPES: &[&str] = &["full_time", "part_time", "contract", "internship"];
pub const WORK_MODES: &[&str] = &["remote", "hybrid", "on_site"];
pub const PAY_UNITS: &[&str] = &["hourly", "annual"];

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Compensation {
    #[serde(default)]
    pub unit: Option<String>,
    #[serde(default)]
    pub min: Option<f64>,
    #[serde(default)]
    pub max: Option<f64>,
}

#[derive(Debug, thiserror::Error)]
pub enum ApplicationMetaError {
    #[error("invalid employment_type `{0}`")]
    InvalidEmploymentType(String),
    #[error("invalid work_mode `{0}`")]
    InvalidWorkMode(String),
    #[error("location is required for hybrid and on-site postings")]
    LocationRequired,
    #[error("invalid compensation unit `{0}`")]
    InvalidPayUnit(String),
    #[error("compensation min must be non-negative")]
    NegativePay,
    #[error("compensation max must be >= min")]
    MaxLessThanMin,
}

pub fn validate_application_meta(
    employment_type: Option<&str>,
    work_mode: Option<&str>,
    location: Option<&str>,
    compensation: &Compensation,
) -> Result<(), ApplicationMetaError> {
    if let Some(et) = employment_type {
        if !EMPLOYMENT_TYPES.contains(&et) {
            return Err(ApplicationMetaError::InvalidEmploymentType(et.to_string()));
        }
    }
    if let Some(wm) = work_mode {
        if !WORK_MODES.contains(&wm) {
            return Err(ApplicationMetaError::InvalidWorkMode(wm.to_string()));
        }
        if (wm == "hybrid" || wm == "on_site")
            && location.map(|s| s.trim().is_empty()).unwrap_or(true)
        {
            return Err(ApplicationMetaError::LocationRequired);
        }
    }
    validate_compensation(compensation)?;
    Ok(())
}

pub fn validate_compensation(c: &Compensation) -> Result<(), ApplicationMetaError> {
    let Some(unit) = c.unit.as_deref() else {
        if c.min.is_some() || c.max.is_some() {
            return Err(ApplicationMetaError::InvalidPayUnit(
                "required when min/max set".into(),
            ));
        }
        return Ok(());
    };
    if !PAY_UNITS.contains(&unit) {
        return Err(ApplicationMetaError::InvalidPayUnit(unit.to_string()));
    }
    if let Some(v) = c.min {
        if v < 0.0 {
            return Err(ApplicationMetaError::NegativePay);
        }
    }
    if let Some(v) = c.max {
        if v < 0.0 {
            return Err(ApplicationMetaError::NegativePay);
        }
    }
    if let (Some(min), Some(max)) = (c.min, c.max) {
        if max < min {
            return Err(ApplicationMetaError::MaxLessThanMin);
        }
    }
    Ok(())
}

/// Annual hours baseline for hourly → yearly display (v1).
pub fn annual_hours_for_employment(employment_type: Option<&str>) -> f64 {
    match employment_type {
        Some("part_time") | Some("internship") => 1040.0,
        _ => 2080.0,
    }
}

pub fn annual_equivalent(c: &Compensation, employment_type: Option<&str>) -> Option<(f64, f64)> {
    if c.unit.as_deref()? != "hourly" {
        return None;
    }
    let hours = annual_hours_for_employment(employment_type);
    let min = c.min? * hours;
    let max = c.max.unwrap_or(c.min?) * hours;
    Some((min, max))
}

/// Human-readable compensation for ledger/UI.
pub fn format_compensation(c: &Compensation, employment_type: Option<&str>) -> String {
    if c.unit.is_none() && c.min.is_none() {
        return String::new();
    }
    let unit = c.unit.as_deref().unwrap_or("annual");
    let min = c.min;
    let max = c.max;
    let range = match (min, max) {
        (Some(a), Some(b)) if (a - b).abs() > f64::EPSILON => {
            format!("${a:.0}–${b:.0}")
        }
        (Some(a), _) => format!("${a:.0}"),
        _ => return String::new(),
    };
    let suffix = if unit == "hourly" { "/hr" } else { "/yr" };
    let mut out = format!("{range}{suffix}");
    if let Some((amin, amax)) = annual_equivalent(c, employment_type) {
        if (amin - amax).abs() > f64::EPSILON {
            out.push_str(&format!(" (~${amin:.0}–${amax:.0}/yr equiv.)"));
        } else {
            out.push_str(&format!(" (~${amin:.0}/yr equiv.)"));
        }
    }
    out
}
