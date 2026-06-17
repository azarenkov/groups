use validator::ValidationError;

pub fn validate_username(value: &str) -> Result<(), ValidationError> {
    if value.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        Ok(())
    } else {
        Err(ValidationError::new("username_charset"))
    }
}
