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
