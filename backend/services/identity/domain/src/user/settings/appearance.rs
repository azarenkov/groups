#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Theme {
    Light,
    #[default]
    Dark,
    System,
}

impl Theme {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "light" => Some(Self::Light),
            "dark" => Some(Self::Dark),
            "system" => Some(Self::System),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Light => "light",
            Self::Dark => "dark",
            Self::System => "system",
        }
    }
}

pub const CHAT_FONT_SIZE_RANGE: std::ops::RangeInclusive<i16> = 12..=18;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum AppearanceError {
    #[error("Invalid theme")]
    InvalidTheme,
    #[error("Invalid accent color")]
    InvalidAccent,
    #[error("Chat font size out of range")]
    InvalidFontSize,
}

#[derive(Debug, Clone, Default)]
pub struct AppearancePatch {
    pub theme: Option<String>,
    pub accent: Option<String>,
    pub chat_font_size: Option<i16>,
    pub reduce_motion: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppearanceSettings {
    theme: Theme,
    accent: String,
    chat_font_size: i16,
    reduce_motion: bool,
}

impl Default for AppearanceSettings {
    fn default() -> Self {
        Self {
            theme: Theme::Dark,
            accent: String::new(),
            chat_font_size: 14,
            reduce_motion: false,
        }
    }
}

fn valid_accent(accent: &str) -> bool {
    accent.is_empty()
        || (accent.len() == 7
            && accent.starts_with('#')
            && accent[1..].chars().all(|c| c.is_ascii_hexdigit()))
}

impl AppearanceSettings {
    pub fn restore(theme: &str, accent: String, chat_font_size: i16, reduce_motion: bool) -> Self {
        Self {
            theme: Theme::parse(theme).unwrap_or_default(),
            accent,
            chat_font_size,
            reduce_motion,
        }
    }

    pub fn apply(&self, patch: &AppearancePatch) -> Result<Self, AppearanceError> {
        let mut next = self.clone();
        if let Some(theme) = patch.theme.as_deref() {
            next.theme = Theme::parse(theme).ok_or(AppearanceError::InvalidTheme)?;
        }
        if let Some(accent) = patch.accent.as_deref() {
            if !valid_accent(accent) {
                return Err(AppearanceError::InvalidAccent);
            }
            next.accent = accent.to_string();
        }
        if let Some(size) = patch.chat_font_size {
            if !CHAT_FONT_SIZE_RANGE.contains(&size) {
                return Err(AppearanceError::InvalidFontSize);
            }
            next.chat_font_size = size;
        }
        if let Some(reduce_motion) = patch.reduce_motion {
            next.reduce_motion = reduce_motion;
        }
        Ok(next)
    }

    pub fn theme(&self) -> Theme {
        self.theme
    }

    pub fn accent(&self) -> &str {
        &self.accent
    }

    pub fn chat_font_size(&self) -> i16 {
        self.chat_font_size
    }

    pub fn reduce_motion(&self) -> bool {
        self.reduce_motion
    }
}
