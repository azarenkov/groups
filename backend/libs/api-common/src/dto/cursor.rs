use domain::pagination::{KeysetCursor, keyset_limit};
use serde::Deserialize;

#[derive(Debug, Deserialize, Default)]
pub struct CursorQuery {
    pub cursor: Option<String>,
    pub limit: Option<u32>,
}

#[derive(Debug)]
pub struct InvalidCursor;

impl CursorQuery {
    pub fn keyset(&self) -> Result<(Option<KeysetCursor>, i64), InvalidCursor> {
        let cursor = match self.cursor.as_deref().filter(|c| !c.is_empty()) {
            Some(raw) => Some(KeysetCursor::parse(raw).ok_or(InvalidCursor)?),
            None => None,
        };
        Ok((cursor, keyset_limit(self.limit)))
    }
}
