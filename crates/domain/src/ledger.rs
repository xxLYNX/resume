use crate::{format_compensation, Compensation};

#[derive(Debug, Clone)]
pub struct ApplicationLedgerRow {
    pub company: String,
    pub title: String,
    pub phase: String,
    pub status: String,
    pub employment_type: Option<String>,
    pub work_mode: Option<String>,
    pub location: Option<String>,
    pub compensation: Compensation,
    pub posting_url: Option<String>,
    pub updated_at: String,
    pub submission_count: u32,
}

pub fn render_ledger_tex(rows: &[ApplicationLedgerRow], generated_at: &str) -> String {
    let mut body = String::from(
        r"\documentclass[10pt]{article}
\usepackage[margin=0.6in]{geometry}
\usepackage{longtable}
\usepackage{array}
\usepackage{hyperref}
\hypersetup{hidelinks}
\pagestyle{empty}
\begin{document}
{\large\textbf{jobdesk application ledger}}\hfill{\small generated #GEN#}\\[0.5em]
\begin{small}
\begin{longtable}{@{}p{0.11\textwidth}p{0.13\textwidth}p{0.06\textwidth}p{0.08\textwidth}p{0.07\textwidth}p{0.07\textwidth}p{0.09\textwidth}p{0.14\textwidth}p{0.08\textwidth}p{0.04\textwidth}@{}}
\textbf{Company} & \textbf{Title} & \textbf{Phase} & \textbf{Pipeline} & \textbf{Emp.} & \textbf{Mode} & \textbf{Location} & \textbf{Pay} & \textbf{Updated} & \textbf{\#} \\
\hline
\endhead
",
    );
    body = body.replace("#GEN#", &latex_escape(generated_at));

    for row in rows {
        let pay = format_compensation(&row.compensation, row.employment_type.as_deref());
        let updated = short_date(&row.updated_at);
        let line = format!(
            "{} & {} & {} & {} & {} & {} & {} & {} & {} & {} \\\\
",
            latex_escape(&row.company),
            latex_escape(&row.title),
            latex_escape(&row.phase),
            latex_escape(&row.status),
            latex_escape(&label_snake(row.employment_type.as_deref())),
            latex_escape(&label_snake(row.work_mode.as_deref())),
            latex_escape(row.location.as_deref().unwrap_or("—")),
            latex_escape(if pay.is_empty() { "—" } else { &pay }),
            latex_escape(&updated),
            row.submission_count,
        );
        body.push_str(&line);
    }

    if rows.is_empty() {
        body.push_str(r"\multicolumn{10}{l}{No applications yet.} \\");
    }

    body.push_str(
        r"\end{longtable}
\end{small}
\end{document}
",
    );
    body
}

fn label_snake(s: Option<&str>) -> String {
    match s {
        Some(x) if !x.is_empty() => x.replace('_', " "),
        _ => "—".to_string(),
    }
}

fn short_date(rfc3339: &str) -> String {
    rfc3339.chars().take(10).collect()
}

pub fn latex_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
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
            _ => out.push(c),
        }
    }
    out
}
