use domain::{pagination::MAX_PER_PAGE, pagination::MIN_PAGES, pagination::MIN_PER_PAGE, pagination::Pagination};

#[derive(Debug, thiserror::Error)]
pub enum PaginationError {
    #[error("page must be >= 1")]
    InvalidPage,
    #[error("per_page must be between 1 and {0}")]
    InvalidPerPage(u32),
}

pub fn parse(page: u32, per_page: u32) -> Result<Pagination, PaginationError> {
    if page < MIN_PAGES {
        return Err(PaginationError::InvalidPage);
    }
    if !(MIN_PER_PAGE..=MAX_PER_PAGE).contains(&per_page) {
        return Err(PaginationError::InvalidPerPage(MAX_PER_PAGE));
    }
    Pagination::new(page, per_page).ok_or(PaginationError::InvalidPage)
}
