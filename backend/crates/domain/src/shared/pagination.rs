pub const MAX_PER_PAGE: u32 = 50;
pub const MIN_PER_PAGE: u32 = 1;
pub const MIN_PAGES: u32 = 1;
pub const DEFAULT_PER_PAGE: u32 = 20;
pub const DEFAULT_PAGE: u32 = 1;

#[derive(Debug, thiserror::Error)]
pub enum PaginationError {
    #[error("page must be >= 1")]
    InvalidPage,
    #[error("per_page must be between 1 and {0}")]
    InvalidPerPage(u32),
}

#[derive(Debug, Clone, Copy)]
pub struct Pagination {
    page: u32,
    per_page: u32,
}

impl Pagination {
    pub fn new(page: u32, per_page: u32) -> Result<Self, PaginationError> {
        if page < MIN_PER_PAGE {
            return Err(PaginationError::InvalidPage);
        }
        if per_page < MIN_PER_PAGE || per_page > MAX_PER_PAGE {
            return Err(PaginationError::InvalidPerPage(MAX_PER_PAGE));
        }
        Ok(Self { page, per_page })
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
