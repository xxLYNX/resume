/// Stable id segment from a catalog label (tool example or similar).
pub fn slugify_label(label: &str) -> String {
    let mut out = String::new();
    let mut prev_hyphen = false;
    for c in label.trim().to_lowercase().chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c);
            prev_hyphen = false;
        } else if !prev_hyphen && !out.is_empty() {
            out.push('-');
            prev_hyphen = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    if out.is_empty() {
        "unknown".to_string()
    } else {
        out
    }
}

pub fn normalize_match_key(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugify_examples() {
        assert_eq!(slugify_label("Python"), "python");
        assert_eq!(slugify_label("Adobe Systems Adobe Acrobat"), "adobe-systems-adobe-acrobat");
    }
}
