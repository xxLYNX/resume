use serde::{Deserialize, Serialize};
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

/// Résumé-style string for LaTeX templates (legacy `date` field).
pub fn format_date_range(range: &DateRange) -> Result<String, DateError> {
    let start = format_calendar_date(&range.start)?;
    let end = match &range.end {
        DateRangeEnd::Present => "Present".to_string(),
        DateRangeEnd::Date { date } => format_calendar_date(date)?,
    };
    Ok(format!("{start} -- {end}"))
}

const MONTH_ABBR: [&str; 12] = [
    "Jan.", "Feb.", "Mar.", "Apr.", "May", "Jun.", "Jul.", "Aug.", "Sep.", "Oct.", "Nov.", "Dec.",
];

pub fn format_calendar_date(d: &CalendarDate) -> Result<String, DateError> {
    d.to_time_date()?;
    let month = MONTH_ABBR
        .get((d.month as usize).saturating_sub(1))
        .ok_or(DateError::InvalidMonth(d.month))?;
    Ok(format!("{month} {}", d.year))
}

/// If `date_range` is set, compute legacy `date` string for templates.
pub fn enrich_payload_dates(value: &mut serde_json::Value) {
    let Some(obj) = value.as_object_mut() else {
        return;
    };
    let Some(dr) = obj.get("date_range") else {
        return;
    };
    let Ok(range) = serde_json::from_value::<DateRange>(dr.clone()) else {
        return;
    };
    if let Ok(formatted) = format_date_range(&range) {
        obj.insert("date".to_string(), serde_json::Value::String(formatted));
    }
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
        let s = format_date_range(&range).unwrap();
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
            format_date_range(&range).unwrap(),
            "Sep. 2020 -- Present"
        );
    }
}
