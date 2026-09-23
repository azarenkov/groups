use ports::text_moderation::{TextModeration, TextModerationError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommunityTextField {
    Name,
    Description,
}

pub async fn ensure_community_text_allowed<TM: TextModeration + ?Sized>(
    moderation: &TM,
    name: &str,
    description: Option<&str>,
) -> Result<(), (CommunityTextField, TextModerationError)> {
    let name = name.trim();
    if !name.is_empty() {
        moderation
            .ensure_allowed(name)
            .await
            .map_err(|e| (CommunityTextField::Name, e))?;
    }
    if let Some(description) = description.map(str::trim).filter(|d| !d.is_empty()) {
        moderation
            .ensure_allowed(description)
            .await
            .map_err(|e| (CommunityTextField::Description, e))?;
    }
    Ok(())
}
