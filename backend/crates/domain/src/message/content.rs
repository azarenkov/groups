use serde::Serialize;

use crate::message::MessageError;

const MAX_CONTENT_LENGTH: usize = 4000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MessageContent(String);

impl MessageContent {
    pub fn new(value: String) -> Result<Self, MessageError> {
        let trimmed = value.trim().to_string();
        if trimmed.is_empty() {
            return Err(MessageError::ContentEmpty);
        }
        if trimmed.chars().count() > MAX_CONTENT_LENGTH {
            return Err(MessageError::ContentTooLong);
        }
        Ok(Self(trimmed))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_string(self) -> String {
        self.0
    }
}
