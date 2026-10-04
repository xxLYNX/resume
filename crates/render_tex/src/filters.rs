use domain::{format_date_range, DateRange, MonthDisplay};
use minijinja::{Environment, Error, ErrorKind, Value};

pub fn register_resume_filters(env: &mut Environment<'_>, month_display: MonthDisplay) {
    env.add_filter("format_date_range", move |value: Value| -> Result<String, Error> {
        format_date_range_filter(value, month_display)
    });
    env.add_filter("latex_escape", latex_escape_filter);
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
