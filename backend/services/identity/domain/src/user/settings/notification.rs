#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MentionLevel {
    #[default]
    All,
    Mentions,
    None,
}

impl MentionLevel {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "all" => Some(Self::All),
            "mentions" => Some(Self::Mentions),
            "none" => Some(Self::None),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Mentions => "mentions",
            Self::None => "none",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimeOfDay(String);

impl TimeOfDay {
    pub fn parse(value: &str) -> Result<Self, NotificationSettingsError> {
        let bytes = value.as_bytes();
        if bytes.len() != 5 || bytes[2] != b':' {
            return Err(NotificationSettingsError::InvalidTime);
        }
        let digits = [bytes[0], bytes[1], bytes[3], bytes[4]];
        if digits.iter().any(|b| !b.is_ascii_digit()) {
            return Err(NotificationSettingsError::InvalidTime);
        }
        let hours = (bytes[0] - b'0') * 10 + (bytes[1] - b'0');
        let minutes = (bytes[3] - b'0') * 10 + (bytes[4] - b'0');
        if hours >= 24 || minutes >= 60 {
            return Err(NotificationSettingsError::InvalidTime);
        }
        Ok(Self(value.to_string()))
    }

    pub fn restore(value: String) -> Self {
        Self(value)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum NotificationSettingsError {
    #[error("Invalid mention level")]
    InvalidMentions,
    #[error("Invalid time of day")]
    InvalidTime,
}

#[derive(Debug, Clone, Default)]
pub struct NotificationSettingsPatch {
    pub desktop_enabled: Option<bool>,
    pub sounds_enabled: Option<bool>,
    pub mentions: Option<String>,
    pub quiet_hours_enabled: Option<bool>,
    pub quiet_start: Option<String>,
    pub quiet_end: Option<String>,
    pub calls_enabled: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotificationSettings {
    desktop_enabled: bool,
    sounds_enabled: bool,
    mentions: MentionLevel,
    quiet_hours_enabled: bool,
    quiet_start: TimeOfDay,
    quiet_end: TimeOfDay,
    calls_enabled: bool,
}

impl Default for NotificationSettings {
    fn default() -> Self {
        Self {
            desktop_enabled: true,
            sounds_enabled: true,
            mentions: MentionLevel::All,
            quiet_hours_enabled: false,
            quiet_start: TimeOfDay("22:00".to_string()),
            quiet_end: TimeOfDay("08:00".to_string()),
            calls_enabled: true,
        }
    }
}

impl NotificationSettings {
    #[allow(clippy::too_many_arguments)]
    pub fn restore(
        desktop_enabled: bool,
        sounds_enabled: bool,
        mentions: &str,
        quiet_hours_enabled: bool,
        quiet_start: String,
        quiet_end: String,
        calls_enabled: bool,
    ) -> Self {
        Self {
            desktop_enabled,
            sounds_enabled,
            mentions: MentionLevel::parse(mentions).unwrap_or_default(),
            quiet_hours_enabled,
            quiet_start: TimeOfDay::restore(quiet_start),
            quiet_end: TimeOfDay::restore(quiet_end),
            calls_enabled,
        }
    }

    pub fn apply(
        &self,
        patch: &NotificationSettingsPatch,
    ) -> Result<Self, NotificationSettingsError> {
        let mut next = self.clone();
        if let Some(enabled) = patch.desktop_enabled {
            next.desktop_enabled = enabled;
        }
        if let Some(enabled) = patch.sounds_enabled {
            next.sounds_enabled = enabled;
        }
        if let Some(level) = patch.mentions.as_deref() {
            next.mentions =
                MentionLevel::parse(level).ok_or(NotificationSettingsError::InvalidMentions)?;
        }
        if let Some(enabled) = patch.quiet_hours_enabled {
            next.quiet_hours_enabled = enabled;
        }
        if let Some(start) = patch.quiet_start.as_deref() {
            next.quiet_start = TimeOfDay::parse(start)?;
        }
        if let Some(end) = patch.quiet_end.as_deref() {
            next.quiet_end = TimeOfDay::parse(end)?;
        }
        if let Some(enabled) = patch.calls_enabled {
            next.calls_enabled = enabled;
        }
        Ok(next)
    }

    pub fn desktop_enabled(&self) -> bool {
        self.desktop_enabled
    }

    pub fn sounds_enabled(&self) -> bool {
        self.sounds_enabled
    }

    pub fn mentions(&self) -> MentionLevel {
        self.mentions
    }

    pub fn quiet_hours_enabled(&self) -> bool {
        self.quiet_hours_enabled
    }

    pub fn quiet_start(&self) -> &TimeOfDay {
        &self.quiet_start
    }

    pub fn quiet_end(&self) -> &TimeOfDay {
        &self.quiet_end
    }

    pub fn calls_enabled(&self) -> bool {
        self.calls_enabled
    }
}
