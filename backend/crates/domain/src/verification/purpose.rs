use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationPurpose {
    Registration,
    PasswordReset,
    EmailChange,
    PhoneChange,
}

impl VerificationPurpose {
    pub fn is_registration(&self) -> bool {
        matches!(self, VerificationPurpose::Registration)
    }
}
