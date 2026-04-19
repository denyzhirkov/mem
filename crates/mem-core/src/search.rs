use crate::error::Result;
use crate::types::{SearchHit, SearchParams};
use crate::vault::open_db;
use std::path::Path;

pub fn run(vault: &Path, params: SearchParams) -> Result<Vec<SearchHit>> {
    let db = open_db(vault)?;
    let mut results: Vec<SearchHit> = db.search(&params.query)?.into_iter().map(Into::into).collect();
    if let Some(lim) = params.limit {
        results.truncate(lim as usize);
    }
    Ok(results)
}
