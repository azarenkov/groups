use chrono::{DateTime, Utc};
use domain::{password_hash::PasswordHash, user::id::UserId, user::name::UserName, user::role::UserRole};
use identity_domain::{user::User, user::email::Email, user::emoji_status::EmojiStatus, user::locale::Locale};
use sqlx::{FromRow, types::Uuid};

#[derive(Debug, FromRow)]
pub struct UserDto {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    pub password_hash: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub email_verified_at: Option<DateTime<Utc>>,
    pub phone_verified_at: Option<DateTime<Utc>>,
    pub locale: String,
    pub pending_email: Option<String>,
    pub password_changed_at: Option<DateTime<Utc>>,
    pub disabled_at: Option<DateTime<Utc>>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub bio: String,
    pub show_activity: bool,
    pub timezone: String,
    pub emoji_status_sticker_id: Option<Uuid>,
    pub emoji_status_until: Option<DateTime<Utc>>,
    pub role: String,
}

impl From<UserDto> for User {
    fn from(dto: UserDto) -> Self {
        let locale = dto.locale.parse::<Locale>().unwrap_or_default();
        User::restore(
            UserId::new(dto.id),
            UserName::new(dto.name),
            Email::new(dto.email),
            PasswordHash::new(dto.password_hash),
            dto.created_at,
            dto.updated_at,
            dto.email_verified_at,
            dto.phone_verified_at,
            locale,
            dto.pending_email.map(Email::new),
            dto.password_changed_at,
            dto.disabled_at,
            dto.deleted_at,
            dto.bio,
            dto.show_activity,
            dto.timezone,
            dto.emoji_status_sticker_id
                .map(|sticker_id| EmojiStatus::new(sticker_id, dto.emoji_status_until)),
            dto.role.parse::<UserRole>().unwrap_or_default(),
        )
    }
}
