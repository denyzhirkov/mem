use once_cell::sync::Lazy;
use regex::Regex;

// Tags: must start with a letter or underscore, at least 2 chars (excludes pure numbers like #1)
static TAG_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)(?:^|[^a-zA-Z0-9_])#([a-z_][a-z0-9_-]+)").unwrap()
});

// Links: [[Link Title]]
static LINK_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\[\[(.*?)\]\]").unwrap()
});

/// Extracts normalized, lower-cased tags from markdown text.
pub fn extract_tags(text: &str) -> Vec<String> {
    let mut tags: Vec<String> = TAG_REGEX
        .captures_iter(text)
        .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_lowercase()))
        .collect();
    tags.sort();
    tags.dedup();
    tags
}

/// Extracts wiki-style link targets from markdown text.
pub fn extract_links(text: &str) -> Vec<String> {
    let mut links: Vec<String> = LINK_REGEX
        .captures_iter(text)
        .filter_map(|cap| cap.get(1).map(|m| m.as_str().trim().to_string()))
        .filter(|s| !s.is_empty())
        .collect();
    links.sort();
    links.dedup();
    links
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_tags() {
        let text = "Hello #world and #rust_lang, also check out #World again.";
        let mut tags = extract_tags(text);
        tags.sort(); // Sorting simply to normalize assertion
        assert_eq!(tags, vec!["rust_lang", "world"]);
    }

    #[test]
    fn test_extract_links() {
        let text = "See [[My Note]] and [[Other-Note]] or [[ My Note ]]";
        let links = extract_links(text);
        // "My Note" is deduped since we trim target titles.
        assert_eq!(links, vec!["My Note", "Other-Note"]);
    }
}
