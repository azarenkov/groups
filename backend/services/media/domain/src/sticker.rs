use async_trait::async_trait;
use chrono::{DateTime, Utc};
use domain::user::id::UserId;
use ports::repository::RepositoryError;
use uuid::Uuid;

pub const MAX_STICKERS_PER_PACK: i64 = 120;
pub const MIN_PACK_NAME_CHARS: usize = 3;
pub const MAX_PACK_NAME_CHARS: usize = 64;
pub const MAX_STICKER_EMOJI_BYTES: usize = 32;
pub const MAX_INSTALLED_PACKS: i64 = 200;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sticker {
    pub id: Uuid,
    pub pack_id: Uuid,
    pub storage_key: String,
    pub animated: bool,
    pub width: i32,
    pub height: i32,
    pub emoji: String,
    pub position: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StickerPack {
    pub id: Uuid,
    pub owner_id: UserId,
    pub name: String,
    pub slug: String,
    pub install_count: i32,
    pub sticker_count: i64,
    pub created_at: DateTime<Utc>,
    pub stickers: Vec<Sticker>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StickerRef {
    pub sticker: Sticker,
    pub pack_name: String,
    pub pack_slug: String,
    pub deleted: bool,
}

#[derive(Debug, Clone)]
pub struct NewSticker {
    pub id: Uuid,
    pub storage_key: String,
    pub animated: bool,
    pub width: i32,
    pub height: i32,
    pub emoji: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallOutcome {
    Installed,
    PackNotFound,
    LimitReached,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddStickerOutcome {
    Added,
    PackNotFound,
    PackFull,
}

#[async_trait]
pub trait StickerRepository: Send + Sync {
    async fn count_owned_packs(&self, owner_id: &UserId) -> Result<i64, RepositoryError>;
    async fn create_pack(&self, id: Uuid, owner_id: &UserId, name: &str, slug: &str, max_owned: i64) -> Result<bool, RepositoryError>;
    async fn find_pack(&self, id_or_slug: &str) -> Result<Option<StickerPack>, RepositoryError>;
    async fn rename_pack(&self, pack_id: Uuid, owner_id: &UserId, name: &str) -> Result<bool, RepositoryError>;
    async fn delete_pack(&self, pack_id: Uuid, owner_id: &UserId) -> Result<bool, RepositoryError>;
    async fn add_sticker(&self, pack_id: Uuid, owner_id: &UserId, sticker: &NewSticker) -> Result<AddStickerOutcome, RepositoryError>;
    async fn delete_sticker(&self, pack_id: Uuid, sticker_id: Uuid, owner_id: &UserId) -> Result<bool, RepositoryError>;
    async fn reorder_stickers(&self, pack_id: Uuid, owner_id: &UserId, order: &[Uuid]) -> Result<bool, RepositoryError>;
    async fn search_packs(&self, query: &str, limit: i64, offset: i64) -> Result<Vec<StickerPack>, RepositoryError>;
    async fn packs_of(&self, user_id: &UserId) -> Result<Vec<StickerPack>, RepositoryError>;
    async fn install(&self, user_id: &UserId, pack_id: Uuid) -> Result<InstallOutcome, RepositoryError>;
    async fn uninstall(&self, user_id: &UserId, pack_id: Uuid) -> Result<bool, RepositoryError>;
    async fn find_sticker(&self, sticker_id: Uuid) -> Result<Option<StickerRef>, RepositoryError>;
    async fn forget_user(&self, user_id: &UserId) -> Result<(), RepositoryError>;
}

pub fn valid_pack_name(name: &str) -> bool {
    let trimmed = name.trim();
    let chars = trimmed.chars().count();
    (MIN_PACK_NAME_CHARS..=MAX_PACK_NAME_CHARS).contains(&chars)
        && !trimmed.chars().any(|c| c.is_control())
}

pub fn valid_pack_slug(slug: &str) -> bool {
    (3..=32).contains(&slug.len())
        && slug
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}

fn is_pictograph(c: char) -> bool {
    matches!(
        c as u32,
        0x00A9
            | 0x00AE
            | 0x203C
            | 0x2049
            | 0x2122
            | 0x2139
            | 0x2194..=0x2199
            | 0x21A9..=0x21AA
            | 0x231A..=0x231B
            | 0x2328
            | 0x23CF
            | 0x23E9..=0x23F3
            | 0x23F8..=0x23FA
            | 0x24C2
            | 0x25AA..=0x25AB
            | 0x25B6
            | 0x25C0
            | 0x25FB..=0x25FE
            | 0x2600..=0x27BF
            | 0x2934..=0x2935
            | 0x2B05..=0x2B07
            | 0x2B1B..=0x2B1C
            | 0x2B50
            | 0x2B55
            | 0x3030
            | 0x303D
            | 0x3297
            | 0x3299
            | 0x1F000..=0x1FAFF
    )
}

fn is_emoji_joiner(c: char) -> bool {
    matches!(c as u32, 0x20 | 0x200D | 0xFE0F | 0x20E3 | 0xE0020..=0xE007F)
}

pub fn valid_sticker_emoji(emoji: &str) -> bool {
    let trimmed = emoji.trim();
    !trimmed.is_empty()
        && trimmed.len() <= MAX_STICKER_EMOJI_BYTES
        && trimmed.chars().any(is_pictograph)
        && trimmed.chars().all(|c| is_pictograph(c) || is_emoji_joiner(c))
}

pub fn max_owned_packs(tier: &str) -> i64 {
    match tier {
        "premium" => 50,
        "pro" => 10,
        _ => 2,
    }
}

pub fn escape_like(query: &str) -> String {
    let mut out = String::with_capacity(query.len());
    for c in query.trim().chars() {
        if matches!(c, '%' | '_' | '\\') {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pack_names_need_three_to_sixty_four_visible_characters() {
        assert!(valid_pack_name("Котики"));
        assert!(valid_pack_name("  abc  "));
        assert!(!valid_pack_name("ab"));
        assert!(!valid_pack_name(&"x".repeat(65)));
        assert!(!valid_pack_name("bad\nname"));
    }

    #[test]
    fn slugs_are_lowercase_latin_digits_and_underscores() {
        assert!(valid_pack_slug("cats_2026"));
        assert!(!valid_pack_slug("Cats"));
        assert!(!valid_pack_slug("ca"));
        assert!(!valid_pack_slug("коты"));
        assert!(!valid_pack_slug("cats-and-dogs"));
    }

    #[test]
    fn sticker_emoji_must_be_emoji_like() {
        assert!(valid_sticker_emoji("😺"));
        assert!(valid_sticker_emoji("👨‍👩‍👧 ❤️"));
        assert!(!valid_sticker_emoji(""));
        assert!(!valid_sticker_emoji("cat"));
        assert!(!valid_sticker_emoji(&"😺".repeat(9)));
        assert!(valid_sticker_emoji("🇷🇺"));
        assert!(valid_sticker_emoji("👍🏽"));
        for text in ["!!!", "---", "()", "😺a", "ок", "\u{200D}"] {
            assert!(!valid_sticker_emoji(text), "{text}");
        }
    }

    #[test]
    fn plan_decides_how_many_packs_one_can_own() {
        assert_eq!(max_owned_packs("free"), 2);
        assert_eq!(max_owned_packs("pro"), 10);
        assert_eq!(max_owned_packs("premium"), 50);
    }

    #[test]
    fn like_wildcards_are_escaped() {
        assert_eq!(escape_like(" 100%_\\ "), "100\\%\\_\\\\");
    }
}
