#[derive(Debug)]
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
