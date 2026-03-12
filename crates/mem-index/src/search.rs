use crate::db::{IndexDb, IndexError};
use mem_domain::{NoteId, SearchResult};
use std::collections::HashSet;

impl IndexDb {
    pub fn search(&self, query: &str) -> Result<Vec<SearchResult>, IndexError> {
        let query = query.trim();
        if query.len() < 2 {
            return Ok(vec![]);
        }

        // Exact tag search: #tagname → find all notes with this tag
        if let Some(tag_name) = query.strip_prefix('#') {
            if !tag_name.is_empty() {
                return self.search_by_tag(tag_name);
            }
        }

        let mut results: Vec<SearchResult> = Vec::new();
        let mut seen: HashSet<String> = HashSet::new();
        let pattern = format!(
            "%{}%",
            query.replace('%', "\\%").replace('_', "\\_")
        );

        // 1. Title matches (highest priority) — LIKE substring, case-insensitive
        let mut stmt = self.conn().prepare(
            "SELECT n.id, n.title
             FROM notes n
             WHERE n.title LIKE ?1 ESCAPE '\\'
             ORDER BY n.updated_at DESC
             LIMIT 20"
        )?;
        let iter = stmt.query_map([&pattern], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        for r in iter.flatten() {
            let (id, title) = r;
            if seen.insert(id.clone()) {
                let excerpt = highlight_match(&title, query);
                results.push(SearchResult {
                    note_id: NoteId(id),
                    title,
                    excerpt,
                    match_kind: "title".to_string(),
                    score: 2.0,
                });
            }
        }

        // 2. Content matches — LIKE on FTS content column for reliable substring search
        let mut stmt = self.conn().prepare(
            "SELECT f.note_id, n.title, f.content
             FROM fts_notes f
             JOIN notes n ON n.id = f.note_id
             WHERE f.content LIKE ?1 ESCAPE '\\'
             ORDER BY n.updated_at DESC
             LIMIT 30"
        )?;
        let iter = stmt.query_map([&pattern], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?))
        })?;
        for r in iter.flatten() {
            let (id, title, content) = r;
            if seen.insert(id.clone()) {
                let excerpt = extract_snippet(&content, query, 60);
                results.push(SearchResult {
                    note_id: NoteId(id),
                    title,
                    excerpt,
                    match_kind: "content".to_string(),
                    score: 1.0,
                });
            }
        }

        // 3. Tag matches — LIKE substring
        let mut stmt = self.conn().prepare(
            "SELECT n.id, n.title, nt.tag_name
             FROM note_tags nt
             JOIN notes n ON n.id = nt.note_id
             WHERE nt.tag_name LIKE ?1 ESCAPE '\\'
             ORDER BY n.updated_at DESC
             LIMIT 20"
        )?;
        let iter = stmt.query_map([&pattern], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?))
        })?;
        for r in iter.flatten() {
            let (id, title, tag) = r;
            if seen.insert(id.clone()) {
                results.push(SearchResult {
                    note_id: NoteId(id),
                    title,
                    excerpt: format!("<m>#{}</m>", tag),
                    match_kind: "tag".to_string(),
                    score: 0.5,
                });
            }
        }

        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(50);
        Ok(results)
    }

    /// Exact tag search: returns all notes that have this tag (case-insensitive)
    fn search_by_tag(&self, tag_name: &str) -> Result<Vec<SearchResult>, IndexError> {
        let tag_lower = tag_name.to_lowercase();
        let mut stmt = self.conn().prepare(
            "SELECT n.id, n.title, nt.tag_name
             FROM note_tags nt
             JOIN notes n ON n.id = nt.note_id
             WHERE LOWER(nt.tag_name) = LOWER(?1)
             ORDER BY n.updated_at DESC
             LIMIT 50"
        )?;
        let iter = stmt.query_map([&tag_lower], |row| {
            Ok(SearchResult {
                note_id: NoteId(row.get(0)?),
                title: row.get(1)?,
                excerpt: {
                    let tag: String = row.get(2)?;
                    format!("<m>#{}</m>", tag)
                },
                match_kind: "tag".to_string(),
                score: 1.0,
            })
        })?;
        iter.collect::<Result<Vec<_>, _>>().map_err(|e| e.into())
    }
}

/// Highlight first match of `query` in `text` with <m>...</m> (case-insensitive)
fn highlight_match(text: &str, query: &str) -> String {
    let text_lower = text.to_lowercase();
    let query_lower = query.to_lowercase();
    if let Some(pos) = text_lower.find(&query_lower) {
        let before = &text[..pos];
        let matched = &text[pos..pos + query.len()];
        let after = &text[pos + query.len()..];
        format!("{}<m>{}</m>{}", before, matched, after)
    } else {
        text.to_string()
    }
}

/// Extract snippet around first match with <m>...</m> highlight
fn extract_snippet(content: &str, query: &str, context_chars: usize) -> String {
    let content_lower = content.to_lowercase();
    let query_lower = query.to_lowercase();

    let byte_pos = match content_lower.find(&query_lower) {
        Some(p) => p,
        None => return String::new(),
    };

    let raw_start = byte_pos.saturating_sub(context_chars);
    let raw_end = (byte_pos + query.len() + context_chars).min(content.len());

    // Snap to valid UTF-8 char boundaries
    let start = snap_char(content, raw_start, false);
    let end = snap_char(content, raw_end, true);

    let snippet = &content[start..end];
    let (trimmed, trim_offset) = trim_to_words(snippet, start > 0, end < content.len());

    let prefix = if start > 0 { "…" } else { "" };
    let suffix = if end < content.len() { "…" } else { "" };

    let m_start = byte_pos - start;
    let m_end = m_start + query.len();

    if m_start < trim_offset || m_end > trim_offset + trimmed.len() {
        return format!("{}{}{}", prefix, trimmed.replace('\n', " "), suffix);
    }

    let adj_s = m_start - trim_offset;
    let adj_e = m_end - trim_offset;

    format!(
        "{}{}<m>{}</m>{}{}",
        prefix,
        trimmed[..adj_s].replace('\n', " "),
        trimmed[adj_s..adj_e].replace('\n', " "),
        trimmed[adj_e..].replace('\n', " "),
        suffix,
    )
}

fn snap_char(s: &str, pos: usize, forward: bool) -> usize {
    if pos >= s.len() { return s.len(); }
    if s.is_char_boundary(pos) { return pos; }
    if forward {
        (pos..s.len()).find(|&i| s.is_char_boundary(i)).unwrap_or(s.len())
    } else {
        (0..=pos).rev().find(|&i| s.is_char_boundary(i)).unwrap_or(0)
    }
}

fn trim_to_words(snippet: &str, trim_start: bool, trim_end: bool) -> (&str, usize) {
    let s = if trim_start {
        snippet.find(char::is_whitespace)
            .and_then(|i| snippet[i..].find(|c: char| !c.is_whitespace()).map(|j| i + j))
            .unwrap_or(0)
    } else {
        0
    };

    let e = if trim_end {
        snippet.rfind(char::is_whitespace).unwrap_or(snippet.len())
    } else {
        snippet.len()
    };

    if s >= e { return (snippet, 0); }
    (&snippet[s..e], s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn highlight_case_insensitive() {
        assert_eq!(highlight_match("Hello World", "world"), "Hello <m>World</m>");
        assert_eq!(highlight_match("SQL query", "sql"), "<m>SQL</m> query");
    }

    #[test]
    fn highlight_substring() {
        assert_eq!(highlight_match("postgresql", "sql"), "postgre<m>sql</m>");
    }

    #[test]
    fn highlight_no_match() {
        assert_eq!(highlight_match("hello", "xyz"), "hello");
    }

    #[test]
    fn snippet_basic() {
        let content = "This is a test document about SQL databases and queries.";
        let snippet = extract_snippet(content, "SQL", 15);
        assert!(snippet.contains("<m>SQL</m>"));
    }

    #[test]
    fn snippet_at_start() {
        let content = "SQL is great for data";
        let snippet = extract_snippet(content, "SQL", 20);
        assert!(snippet.starts_with("<m>SQL</m>"));
    }

    #[test]
    fn snippet_case_insensitive() {
        let content = "Working with PostgreSQL is fun";
        let snippet = extract_snippet(content, "sql", 15);
        assert!(snippet.contains("<m>SQL</m>"));
    }
}
