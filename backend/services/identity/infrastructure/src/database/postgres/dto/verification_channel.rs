use identity_domain::verification::channel::VerificationChannel;

#[derive(Debug, sqlx::Type)]
#[sqlx(type_name = "verification_channel", rename_all = "snake_case")]
pub enum VerificationChannelDto {
    Email,
    Phone,
}

impl From<&VerificationChannel> for VerificationChannelDto {
    fn from(value: &VerificationChannel) -> Self {
        match value {
            VerificationChannel::Email => Self::Email,
            VerificationChannel::Phone => Self::Phone,
        }
    }
}

impl From<VerificationChannelDto> for VerificationChannel {
    fn from(value: VerificationChannelDto) -> Self {
        match value {
            VerificationChannelDto::Email => Self::Email,
            VerificationChannelDto::Phone => Self::Phone,
        }
    }
}
