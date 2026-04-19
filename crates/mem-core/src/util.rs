pub fn generate_slug(title: &str) -> String {
    title
        .to_lowercase()
        .replace(|c: char| !c.is_alphanumeric(), "-")
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slug_basic() {
        assert_eq!(generate_slug("Hello World"), "hello-world");
        assert_eq!(generate_slug("  Multi  -- Spaces  "), "multi-spaces");
        assert_eq!(generate_slug("Тест 123"), "тест-123");
    }
}
