use identity_domain::verification::purpose::VerificationPurpose;

#[derive(Debug, sqlx::Type)]
#[sqlx(type_name = "verification_purpose", rename_all = "snake_case")]
pub enum VerificationPurposeDto {
    Registration,
    PasswordReset,
    EmailChange,
    PhoneChange,
}

impl From<&VerificationPurpose> for VerificationPurposeDto {
    fn from(value: &VerificationPurpose) -> Self {
        match value {
            VerificationPurpose::Registration => Self::Registration,
            VerificationPurpose::PasswordReset => Self::PasswordReset,
            VerificationPurpose::EmailChange => Self::EmailChange,
            VerificationPurpose::PhoneChange => Self::PhoneChange,
        }
    }
}

impl From<VerificationPurposeDto> for VerificationPurpose {
    fn from(value: VerificationPurposeDto) -> Self {
        match value {
            VerificationPurposeDto::Registration => Self::Registration,
            VerificationPurposeDto::PasswordReset => Self::PasswordReset,
            VerificationPurposeDto::EmailChange => Self::EmailChange,
            VerificationPurposeDto::PhoneChange => Self::PhoneChange,
        }
    }
}
