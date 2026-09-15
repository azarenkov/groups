use chrono::{DateTime, TimeZone, Utc};

pub const MAX_PER_PAGE: u32 = 50;
pub const MIN_PER_PAGE: u32 = 1;
pub const MIN_PAGES: u32 = 1;
pub const DEFAULT_PER_PAGE: u32 = 20;
pub const DEFAULT_PAGE: u32 = 1;

#[derive(Debug, Clone, Copy)]
pub struct Pagination {
    page: u32,
    per_page: u32,
}

impl Pagination {
    pub fn new(page: u32, per_page: u32) -> Option<Self> {
        if page < MIN_PAGES || !(MIN_PER_PAGE..=MAX_PER_PAGE).contains(&per_page) {
            return None;
        }
        Some(Self { page, per_page })
    }
    pub fn offset(&self) -> i64 {
        (self.page as i64 - 1) * self.per_page as i64
    }

    pub fn limit(&self) -> i64 {
        self.per_page as i64
    }

    pub fn page(&self) -> u32 {
        self.page
    }

    pub fn per_page(&self) -> u32 {
        self.per_page
    }
}

#[derive(Debug)]
pub struct PaginatedResult<T> {
    pub items: Vec<T>,
    pub total: i64,
    pub pagination: Pagination,
}

impl<T> PaginatedResult<T> {
    pub fn new(items: Vec<T>, total: i64, pagination: Pagination) -> Self {
        Self {
            items,
            total,
            pagination,
        }
    }
}

pub const DEFAULT_KEYSET_LIMIT: u32 = 50;
pub const MAX_KEYSET_LIMIT: u32 = 200;

pub fn keyset_limit(requested: Option<u32>) -> i64 {
    requested
        .unwrap_or(DEFAULT_KEYSET_LIMIT)
        .clamp(1, MAX_KEYSET_LIMIT) as i64
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeysetCursor {
    pub at: DateTime<Utc>,
    pub key: String,
}

impl KeysetCursor {
    pub fn new(at: DateTime<Utc>, key: impl Into<String>) -> Self {
        Self { at, key: key.into() }
    }

    pub fn encode(&self) -> String {
        format!("{}.{}", self.at.timestamp_micros(), self.key)
    }

    pub fn parse(raw: &str) -> Option<Self> {
        let (micros, key) = raw.split_once('.')?;
        if key.is_empty() {
            return None;
        }
        let micros: i64 = micros.parse().ok()?;
        let at = Utc.timestamp_micros(micros).single()?;
        Some(Self::new(at, key))
    }
}

#[derive(Debug)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub next_cursor: Option<KeysetCursor>,
}

impl<T> Page<T> {
    pub fn from_overfetch(mut items: Vec<T>, limit: i64, cursor_of: impl Fn(&T) -> KeysetCursor) -> Self {
        let limit = usize::try_from(limit).unwrap_or(0);
        if items.len() > limit {
            items.truncate(limit);
            let next_cursor = items.last().map(&cursor_of);
            return Self { items, next_cursor };
        }
        Self { items, next_cursor: None }
    }

    pub fn map<U>(self, f: impl FnMut(T) -> U) -> Page<U> {
        Page {
            items: self.items.into_iter().map(f).collect(),
            next_cursor: self.next_cursor,
        }
    }
}
