#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NoiseMode {
    Off,
    #[default]
    Standard,
    High,
}

impl NoiseMode {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "off" => Some(Self::Off),
            "standard" => Some(Self::Standard),
            "high" => Some(Self::High),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Standard => "standard",
            Self::High => "high",
        }
    }
}

pub const MODIFIER_KEY_CODES: &[&str] = &[
    "ShiftLeft",
    "ShiftRight",
    "ControlLeft",
    "ControlRight",
    "AltLeft",
    "AltRight",
    "MetaLeft",
    "MetaRight",
];

const PTT_KEY_MAX_LEN: usize = 32;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PttKey(String);

impl PttKey {
    pub fn parse(value: &str) -> Result<Self, VoiceSettingsError> {
        if value.is_empty()
            || value.len() > PTT_KEY_MAX_LEN
            || !value.chars().all(|c| c.is_ascii_alphanumeric())
            || MODIFIER_KEY_CODES.contains(&value)
        {
            return Err(VoiceSettingsError::InvalidPttKey);
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

impl Default for PttKey {
    fn default() -> Self {
        Self("KeyM".to_string())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum VoiceSettingsError {
    #[error("Volume must be between 0 and 100")]
    InvalidVolume,
    #[error("Sensitivity must be between 0 and 100")]
    InvalidSensitivity,
    #[error("Invalid noise suppression mode")]
    InvalidNoiseMode,
    #[error("Invalid push-to-talk key")]
    InvalidPttKey,
    #[error("Invalid stream quality")]
    InvalidStreamQuality,
    #[error("Invalid screen content")]
    InvalidScreenContent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ScreenContent {
    #[default]
    Auto,
    Text,
    Motion,
}

impl ScreenContent {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "auto" => Some(Self::Auto),
            "text" => Some(Self::Text),
            "motion" => Some(Self::Motion),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Text => "text",
            Self::Motion => "motion",
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct VoiceSettingsPatch {
    pub mic_volume: Option<i16>,
    pub output_volume: Option<i16>,
    pub noise_mode: Option<String>,
    pub auto_sensitivity: Option<bool>,
    pub sensitivity: Option<i16>,
    pub push_to_talk: Option<bool>,
    pub ptt_key: Option<String>,
    pub stream_quality: Option<String>,
    pub screen_content: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum StreamQuality {
    #[default]
    Auto,
    P720,
    P1080,
    P1440,
    P2160,
}

impl StreamQuality {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::P720 => "720p60",
            Self::P1080 => "1080p60",
            Self::P1440 => "1440p60",
            Self::P2160 => "2160p60",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "auto" => Some(Self::Auto),
            "720p60" => Some(Self::P720),
            "1080p60" => Some(Self::P1080),
            "1440p60" => Some(Self::P1440),
            "2160p60" => Some(Self::P2160),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VoiceSettings {
    mic_volume: i16,
    output_volume: i16,
    noise_mode: NoiseMode,
    auto_sensitivity: bool,
    sensitivity: i16,
    push_to_talk: bool,
    ptt_key: PttKey,
    stream_quality: StreamQuality,
    screen_content: Option<ScreenContent>,
}

impl Default for VoiceSettings {
    fn default() -> Self {
        Self {
            mic_volume: 100,
            output_volume: 100,
            noise_mode: NoiseMode::Standard,
            auto_sensitivity: true,
            sensitivity: 50,
            push_to_talk: false,
            ptt_key: PttKey::default(),
            stream_quality: StreamQuality::default(),
            screen_content: None,
        }
    }
}

fn valid_percent(value: i16) -> bool {
    (0..=100).contains(&value)
}

impl VoiceSettings {
    #[allow(clippy::too_many_arguments)]
    pub fn restore(
        mic_volume: i16,
        output_volume: i16,
        noise_mode: &str,
        auto_sensitivity: bool,
        sensitivity: i16,
        push_to_talk: bool,
        ptt_key: String,
        stream_quality: &str,
        screen_content: Option<&str>,
    ) -> Self {
        Self {
            stream_quality: StreamQuality::parse(stream_quality).unwrap_or_default(),
            screen_content: screen_content.and_then(ScreenContent::parse),
            mic_volume,
            output_volume,
            noise_mode: NoiseMode::parse(noise_mode).unwrap_or_default(),
            auto_sensitivity,
            sensitivity,
            push_to_talk,
            ptt_key: PttKey::restore(ptt_key),
        }
    }

    pub fn apply(&self, patch: &VoiceSettingsPatch) -> Result<Self, VoiceSettingsError> {
        let mut next = self.clone();
        if let Some(volume) = patch.mic_volume {
            if !valid_percent(volume) {
                return Err(VoiceSettingsError::InvalidVolume);
            }
            next.mic_volume = volume;
        }
        if let Some(volume) = patch.output_volume {
            if !valid_percent(volume) {
                return Err(VoiceSettingsError::InvalidVolume);
            }
            next.output_volume = volume;
        }
        if let Some(mode) = patch.noise_mode.as_deref() {
            next.noise_mode = NoiseMode::parse(mode).ok_or(VoiceSettingsError::InvalidNoiseMode)?;
        }
        if let Some(auto) = patch.auto_sensitivity {
            next.auto_sensitivity = auto;
        }
        if let Some(sensitivity) = patch.sensitivity {
            if !valid_percent(sensitivity) {
                return Err(VoiceSettingsError::InvalidSensitivity);
            }
            next.sensitivity = sensitivity;
        }
        if let Some(push_to_talk) = patch.push_to_talk {
            next.push_to_talk = push_to_talk;
        }
        if let Some(key) = patch.ptt_key.as_deref() {
            next.ptt_key = PttKey::parse(key)?;
        }
        if let Some(quality) = patch.stream_quality.as_deref() {
            next.stream_quality =
                StreamQuality::parse(quality).ok_or(VoiceSettingsError::InvalidStreamQuality)?;
        }
        if let Some(content) = patch.screen_content.as_deref() {
            next.screen_content =
                Some(ScreenContent::parse(content).ok_or(VoiceSettingsError::InvalidScreenContent)?);
        }
        Ok(next)
    }

    pub fn mic_volume(&self) -> i16 {
        self.mic_volume
    }

    pub fn output_volume(&self) -> i16 {
        self.output_volume
    }

    pub fn noise_mode(&self) -> NoiseMode {
        self.noise_mode
    }

    pub fn stream_quality(&self) -> StreamQuality {
        self.stream_quality
    }

    pub fn screen_content(&self) -> Option<ScreenContent> {
        self.screen_content
    }

    pub fn auto_sensitivity(&self) -> bool {
        self.auto_sensitivity
    }

    pub fn sensitivity(&self) -> i16 {
        self.sensitivity
    }

    pub fn push_to_talk(&self) -> bool {
        self.push_to_talk
    }

    pub fn ptt_key(&self) -> &PttKey {
        &self.ptt_key
    }
}
