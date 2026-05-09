#[derive(Debug)]
pub enum VerificationPurpose {
    Registration,
    PasswordReset,
    EmailChange,
    PhoneChange,
}
