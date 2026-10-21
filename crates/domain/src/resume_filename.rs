//! Suggested save name for resume PDFs (e.g. job applications, preview download).

/// `firstname-lastname-resume.pdf` from a contact `name` field; ASCII slug, lowercase.
pub fn resume_download_filename(full_name: &str) -> String {
    let parts: Vec<&str> = full_name.split_whitespace().filter(|p| !p.is_empty()).collect();
    match parts.len() {
        0 => "resume.pdf".to_string(),
        1 => format!("{}-resume.pdf", slug_part(parts[0])),
        _ => format!(
            "{}-{}-resume.pdf",
            slug_part(parts[0]),
            slug_part(parts[parts.len() - 1])
        ),
    }
}

fn slug_part(word: &str) -> String {
    let mut out = String::new();
    for c in word.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if c == '-' || c == '_' {
            if !out.ends_with('-') {
                out.push('-');
            }
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    if out.is_empty() {
        "resume".to_string()
    } else {
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_part_name() {
        assert_eq!(
            resume_download_filename("Cullen Kelley"),
            "cullen-kelley-resume.pdf"
        );
    }

    #[test]
    fn single_name() {
        assert_eq!(resume_download_filename("Madonna"), "madonna-resume.pdf");
    }

    #[test]
    fn empty_falls_back() {
        assert_eq!(resume_download_filename("   "), "resume.pdf");
    }
}
