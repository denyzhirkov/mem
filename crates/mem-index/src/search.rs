use crate::db::{IndexDb, IndexError};
use mem_domain::{NoteId, SearchResult};

impl IndexDb {
    pub fn search(&self, query: &str) -> Result<Vec<SearchResult>, IndexError> {
        let query = query.trim();
        if query.is_empty() {
            return Ok(vec![]);
        }

        let mut results: Vec<SearchResult> = Vec::new();
        let mut seen_ids: std::collections::HashSet<String> = std::collections::HashSet::new();

        // 1. FTS5 content search — title + body with snippet
        let fts_query = Self::build_fts_query(query);
        if let Ok(mut stmt) = self.conn().prepare(
            "SELECT f.note_id, n.title,
                    snippet(fts_notes, 2, '<m>', '</m>', '…', 32) as excerpt,
                    rank
             FROM fts_notes f
             JOIN notes n ON n.id = f.note_id
             WHERE fts_notes MATCH ?1
             ORDER BY rank
             LIMIT 30"
        ) {
            if let Ok(iter) = stmt.query_map([&fts_query], |row| {
                let rank: f64 = row.get(3)?;
                Ok(SearchResult {
                    note_id: NoteId(row.get(0)?),
                    title: row.get(1)?,
                    excerpt: row.get(2)?,
                    match_kind: "content".to_string(),
                    score: (-rank) as f32, // FTS5 rank is negative, lower = better
                })
            }) {
                for r in iter.flatten() {
                    seen_ids.insert(r.note_id.0.clone());
                    results.push(r);
                }
            }
        }

        // 2. Tag search — catch tags that FTS might miss
        let pattern = format!("%{}%", query);
        let mut stmt = self.conn().prepare(
            "SELECT n.id, n.title, nt.tag_name
             FROM note_tags nt
             JOIN notes n ON n.id = nt.note_id
             WHERE nt.tag_name LIKE ?1
             ORDER BY n.updated_at DESC
             LIMIT 20"
        )?;
        let iter = stmt.query_map([&pattern], |row| {
            Ok(SearchResult {
                note_id: NoteId(row.get(0)?),
                title: row.get(1)?,
                excerpt: {
                    let tag: String = row.get(2)?;
                    format!("#{}",tag)
                },
                match_kind: "tag".to_string(),
                score: 0.5,
            })
        })?;
        for r in iter.flatten() {
            if !seen_ids.contains(&r.note_id.0) {
                seen_ids.insert(r.note_id.0.clone());
                results.push(r);
            }
        }

        // Sort: higher score first
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(50);

        Ok(results)
    }

    /// Build an FTS5 query from user input.
    /// Wraps each token with * for prefix matching.
    fn build_fts_query(input: &str) -> String {
        input
            .split_whitespace()
            .map(|token| {
                // Escape quotes
                let clean = token.replace('"', "");
                if clean.is_empty() {
                    return String::new();
                }
                format!("\"{}\"*", clean)
            })
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(" ")
    }
}
