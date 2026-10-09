use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;
use time::{Date, Month};

/// Calendar date for résumé entries (day optional when only month/year matter).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalendarDate {
    pub year: i32,
    #[serde(deserialize_with = "deserialize_month")]
    pub month: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub day: Option<u8>,
}

#[derive(Debug, Error)]
pub enum DateError {
    #[error("invalid month `{0}` (expected 1–12)")]
    InvalidMonth(u8),
    #[error("invalid calendar date")]
    InvalidDate(#[from] time::error::ComponentRange),
    #[error("end date must not be before start date")]
    EndBeforeStart,
    #[error("unrecognized legacy date `{0}`")]
    UnrecognizedLegacyDate(String),
}

fn deserialize_month<'de, D>(deserializer: D) -> Result<u8, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let m: u8 = Deserialize::deserialize(deserializer)?;
    if !(1..=12).contains(&m) {
        return Err(serde::de::Error::custom(format!("month must be 1–12, got {m}")));
    }
    Ok(m)
}

impl CalendarDate {
    pub fn to_time_date(&self) -> Result<Date, DateError> {
        if !(1..=12).contains(&self.month) {
            return Err(DateError::InvalidMonth(self.month));
        }
        let month = Month::try_from(self.month).map_err(|_| DateError::InvalidMonth(self.month))?;
        let day = self.day.unwrap_or(1);
        Ok(Date::from_calendar_date(self.year, month, day)?)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DateRangeEnd {
    Present,
    Date { date: CalendarDate },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DateRange {
    pub start: CalendarDate,
    pub end: DateRangeEnd,
}

pub const MONTH_ABBR: [&str; 12] = [
    "Jan.", "Feb.", "Mar.", "Apr.", "May", "Jun.", "Jul.", "Aug.", "Sep.", "Oct.", "Nov.", "Dec.",
];

/// How themes print months in formatted date strings (abbreviations are jobdesk-owned).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MonthDisplay {
    #[default]
    Abbrev,
    Numeric,
}

impl MonthDisplay {
    pub fn parse(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "numeric" | "number" | "num" => Self::Numeric,
            _ => Self::Abbrev,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Abbrev => "abbrev",
            Self::Numeric => "numeric",
        }
    }
}

/// Résumé display string for templates (render-time filter).
pub fn format_date_range(range: &DateRange, month_display: MonthDisplay) -> Result<String, DateError> {
    let start = format_calendar_date(&range.start, month_display)?;
    let end = match &range.end {
        DateRangeEnd::Present => "Present".to_string(),
        DateRangeEnd::Date { date } => format_calendar_date(date, month_display)?,
    };
    Ok(format!("{start} -- {end}"))
}

pub fn format_calendar_date(d: &CalendarDate, month_display: MonthDisplay) -> Result<String, DateError> {
    d.to_time_date()?;
    if !(1..=12).contains(&d.month) {
        return Err(DateError::InvalidMonth(d.month));
    }
    let month_token = match month_display {
        MonthDisplay::Abbrev => MONTH_ABBR[(d.month as usize) - 1].to_string(),
        MonthDisplay::Numeric => d.month.to_string(),
    };
    Ok(format!("{month_token} {}", d.year))
}

pub fn validate_date_range(range: &DateRange) -> Result<(), DateError> {
    range.start.to_time_date()?;
    if let DateRangeEnd::Date { date } = &range.end {
        date.to_time_date()?;
        let start = range.start.to_time_date()?;
        let end = date.to_time_date()?;
        if end < start {
            return Err(DateError::EndBeforeStart);
        }
    }
    Ok(())
}

/// Convert legacy `date` display strings to structured `date_range` and drop `date`.
/// Normalize each row in YAML/JSON list sections (`experience`, `education`, `projects`).
pub fn normalize_list_section_document(section: &str, doc: &mut Value) {
    if !matches!(section, "education" | "experience" | "projects") {
        return;
    }
    let Some(items) = doc.get_mut(section).and_then(|v| v.as_array_mut()) else {
        return;
    };
    for item in items.iter_mut() {
        let _ = normalize_payload_dates(item);
    }
}

pub fn normalize_payload_dates(payload: &mut Value) -> Result<(), DateError> {
    let Some(obj) = payload.as_object_mut() else {
        return Ok(());
    };
    if obj.contains_key("date_range") {
        obj.remove("date");
        return Ok(());
    }
    let Some(Value::String(s)) = obj.get("date").cloned() else {
        return Ok(());
    };
    let range = parse_legacy_date_range(&s)?;
    validate_date_range(&range)?;
    obj.insert(
        "date_range".to_string(),
        serde_json::to_value(&range).expect("DateRange serializes"),
    );
    obj.remove("date");
    Ok(())
}

pub fn parse_legacy_date_range(s: &str) -> Result<DateRange, DateError> {
    let parts: Vec<&str> = s.split(" -- ").collect();
    if parts.len() != 2 {
        return Err(DateError::UnrecognizedLegacyDate(s.to_string()));
    }
    let start = parse_legacy_calendar_token(parts[0].trim())?;
    let end_token = parts[1].trim();
    let end = if end_token.eq_ignore_ascii_case("present") {
        DateRangeEnd::Present
    } else {
        DateRangeEnd::Date {
            date: parse_legacy_calendar_token(end_token)?,
        }
    };
    Ok(DateRange { start, end })
}

fn parse_legacy_calendar_token(s: &str) -> Result<CalendarDate, DateError> {
    let mut iter = s.split_whitespace();
    let month_token = iter
        .next()
        .ok_or_else(|| DateError::UnrecognizedLegacyDate(s.to_string()))?;
    let year_token = iter
        .next()
        .ok_or_else(|| DateError::UnrecognizedLegacyDate(s.to_string()))?;
    if iter.next().is_some() {
        return Err(DateError::UnrecognizedLegacyDate(s.to_string()));
    }
    let month = month_abbr_to_num(month_token.trim_end_matches('.'))?;
    let year: i32 = year_token
        .parse()
        .map_err(|_| DateError::UnrecognizedLegacyDate(s.to_string()))?;
    Ok(CalendarDate {
        year,
        month,
        day: None,
    })
}

fn month_abbr_to_num(token: &str) -> Result<u8, DateError> {
    let found = MONTH_ABBR
        .iter()
        .position(|abbr| abbr.trim_end_matches('.').eq_ignore_ascii_case(token));
    match found {
        Some(i) => Ok((i + 1) as u8),
        None => Err(DateError::UnrecognizedLegacyDate(token.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_range_for_latex() {
        let range = DateRange {
            start: CalendarDate {
                year: 2024,
                month: 6,
                day: None,
            },
            end: DateRangeEnd::Date {
                date: CalendarDate {
                    year: 2024,
                    month: 8,
                    day: None,
                },
            },
        };
        let s = format_date_range(&range, MonthDisplay::Abbrev).unwrap();
        assert_eq!(s, "Jun. 2024 -- Aug. 2024");
    }

    #[test]
    fn present_end() {
        let range = DateRange {
            start: CalendarDate {
                year: 2020,
                month: 9,
                day: None,
            },
            end: DateRangeEnd::Present,
        };
        assert_eq!(
            format_date_range(&range, MonthDisplay::Abbrev).unwrap(),
            "Sep. 2020 -- Present"
        );
    }

    #[test]
    fn numeric_month_display() {
        let range = DateRange {
            start: CalendarDate {
                year: 2024,
                month: 12,
                day: None,
            },
            end: DateRangeEnd::Present,
        };
        assert_eq!(
            format_date_range(&range, MonthDisplay::Numeric).unwrap(),
            "12 2024 -- Present"
        );
    }

    #[test]
    fn parses_legacy_display_string() {
        let range = parse_legacy_date_range("Jun. 2024 -- Aug. 2024").unwrap();
        assert_eq!(range.start.month, 6);
        assert_eq!(range.start.year, 2024);
        assert!(matches!(range.end, DateRangeEnd::Date { .. }));
    }

    #[test]
    fn normalizes_payload_in_place() {
        let mut payload = serde_json::json!({
            "company": "Acme",
            "date": "Sep. 2020 -- Present"
        });
        normalize_payload_dates(&mut payload).unwrap();
        assert!(payload.get("date").is_none());
        let dr = payload.get("date_range").unwrap();
        let range: DateRange = serde_json::from_value(dr.clone()).unwrap();
        assert_eq!(range.start.month, 9);
        assert!(matches!(range.end, DateRangeEnd::Present));
    }
}
