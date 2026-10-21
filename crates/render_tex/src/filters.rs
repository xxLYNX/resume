use domain::{format_date_range, DateRange, MonthDisplay, WorkLocation};
use minijinja::{Environment, Error, ErrorKind, Value};

pub fn register_resume_filters(env: &mut Environment<'_>, month_display: MonthDisplay) {
    env.add_filter("format_date_range", move |value: Value| -> Result<String, Error> {
        format_date_range_filter(value, month_display)
    });
    env.add_filter("latex_escape", latex_escape_filter);
    env.add_filter("url_short", url_short_filter);
    env.add_filter("format_work_location", format_work_location_filter);
    env.add_filter("format_experience_company_line", format_experience_company_line_filter);
}

fn format_work_location_filter(value: Value) -> Result<String, Error> {
    if value.is_none() || value.is_undefined() {
        return Ok(String::new());
    }
    let loc: WorkLocation = serde_json::from_value(serde_json::to_value(&value).map_err(|e| {
        Error::new(
            ErrorKind::InvalidOperation,
            format!("format_work_location: {e}"),
        )
    })?)
    .map_err(|e| Error::new(ErrorKind::InvalidOperation, format!("format_work_location: {e}")))?;
    Ok(format_work_location(&loc))
}

pub fn format_work_location(loc: &WorkLocation) -> String {
    let mut parts = Vec::new();
    if let Some(city) = loc.city.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        parts.push(city.to_string());
    }
    if let Some(state) = loc.state.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        parts.push(state.to_string());
    }
    parts.join(", ")
}

#[derive(serde::Deserialize)]
struct ExperienceCompanyRow {
    company: Option<String>,
    location: Option<WorkLocation>,
}

fn format_experience_company_line_filter(value: Value) -> Result<String, Error> {
    if value.is_none() || value.is_undefined() {
        return Ok(String::new());
    }
    let row: ExperienceCompanyRow = serde_json::from_value(serde_json::to_value(&value).map_err(
        |e| {
            Error::new(
                ErrorKind::InvalidOperation,
                format!("format_experience_company_line: {e}"),
            )
        },
    )?)
    .map_err(|e| {
        Error::new(
            ErrorKind::InvalidOperation,
            format!("format_experience_company_line: {e}"),
        )
    })?;
    let company = row.company.as_deref().unwrap_or("").trim();
    Ok(format_experience_company_line(
        company,
        row.location.as_ref(),
    ))
}

pub fn format_experience_company_line(company: &str, location: Option<&WorkLocation>) -> String {
    let company = company.trim();
    let loc = location
        .map(format_work_location)
        .filter(|s| !s.is_empty());
    match loc {
        Some(l) if !company.is_empty() => format!("{company}, {l}"),
        Some(l) => l,
        None => company.to_string(),
    }
}

/// Escape text for LaTeX body mode (user catalog strings).
pub fn latex_escape(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '\\' && i + 1 < chars.len() {
            let next = chars[i + 1];
            if "%&$#_{}~^\\".contains(next) {
                out.push('\\');
                out.push(next);
                i += 2;
                continue;
            }
        }
        match chars[i] {
            '\\' => out.push_str(r"\textbackslash{}"),
            '&' => out.push_str(r"\&"),
            '%' => out.push_str(r"\%"),
            '$' => out.push_str(r"\$"),
            '#' => out.push_str(r"\#"),
            '_' => out.push_str(r"\_"),
            '{' => out.push_str(r"\{"),
            '}' => out.push_str(r"\}"),
            '~' => out.push_str(r"\textasciitilde{}"),
            '^' => out.push_str(r"\textasciicircum{}"),
            c => out.push(c),
        }
        i += 1;
    }
    out
}

/// Display form for http(s) links in headers (scheme stripped; full URL stays in `\href`).
pub fn url_short(s: &str) -> String {
    let mut t = s.trim();
    if let Some(rest) = t.strip_prefix("https://") {
        t = rest;
    } else if let Some(rest) = t.strip_prefix("http://") {
        t = rest;
    }
    t.trim_end_matches('/').to_string()
}

fn url_short_filter(value: Value) -> Result<String, Error> {
    if value.is_none() || value.is_undefined() {
        return Ok(String::new());
    }
    let s: String = value
        .as_str()
        .map(String::from)
        .ok_or_else(|| Error::new(ErrorKind::InvalidOperation, "url_short expects a string"))?;
    Ok(url_short(&s))
}

fn latex_escape_filter(value: Value) -> Result<String, Error> {
    if value.is_none() || value.is_undefined() {
        return Ok(String::new());
    }
    let s: String = value
        .as_str()
        .map(String::from)
        .ok_or_else(|| Error::new(ErrorKind::InvalidOperation, "latex_escape expects a string"))?;
    Ok(latex_escape(&s))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_short_strips_scheme_and_trailing_slash() {
        assert_eq!(
            url_short("https://linkedin.com/in/foo/"),
            "linkedin.com/in/foo"
        );
        assert_eq!(url_short("http://example.com"), "example.com");
    }
}

fn format_date_range_filter(value: Value, month_display: MonthDisplay) -> Result<String, Error> {
    if value.is_none() || value.is_undefined() {
        return Ok(String::new());
    }
    let range: DateRange = serde_json::from_value(serde_json::to_value(&value).map_err(|e| {
        Error::new(
            ErrorKind::InvalidOperation,
            format!("format_date_range: {e}"),
        )
    })?)
    .map_err(|e| Error::new(ErrorKind::InvalidOperation, format!("format_date_range: {e}")))?;
    format_date_range(&range, month_display).map_err(|e| {
        Error::new(
            ErrorKind::InvalidOperation,
            format!("format_date_range: {e}"),
        )
    })
}
