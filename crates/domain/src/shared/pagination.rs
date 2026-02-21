#[derive(Debug)]
pub struct Pagination {
    page: i64,
    per_page: i64,
}

impl Pagination {
    pub fn new(page: i64, per_page: i64) -> Self {
        Self { page, per_page }
    }

    pub fn offset(&self) -> i64 {
        (self.page - 1) * self.per_page
    }

    pub fn limit(&self) -> i64 {
        self.per_page
    }

    pub fn page(&self) -> i64 {
        self.page
    }

    pub fn per_page(&self) -> i64 {
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
