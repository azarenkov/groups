use std::collections::BTreeMap;
use std::sync::Arc;

use domain::user::id::UserId;
use identity_domain::user::repository::UserRepository;
use ports::repository::RepositoryError;

pub const HOTKEY_ACTIONS: &[(&str, &str)] = &[
    ("quickSwitcher", "Mod+K"),
    ("prevChannel", "Alt+ArrowUp"),
    ("nextChannel", "Alt+ArrowDown"),
    ("historyBack", "Mod+BracketLeft"),
    ("historyForward", "Mod+BracketRight"),
    ("openSettings", "Mod+Comma"),
    ("editLastMessage", "ArrowUp"),
    ("toggleMute", "Mod+Shift+M"),
    ("leaveVoice", "Mod+Shift+L"),
];

fn known_action(action: &str) -> bool {
    HOTKEY_ACTIONS.iter().any(|(id, _)| *id == action)
}

fn defaults_json() -> String {
    let defaults: BTreeMap<&str, &str> = HOTKEY_ACTIONS.iter().copied().collect();
    serde_json::to_string(&defaults).unwrap_or_else(|_| "{}".to_string())
}

const MODIFIERS: &[&str] = &["Mod", "Ctrl", "Alt", "Shift"];

const NAMED_KEYS: &[&str] = &[
    "ArrowUp",
    "ArrowDown",
    "ArrowLeft",
    "ArrowRight",
    "Escape",
    "Space",
    "Enter",
    "Tab",
    "Backspace",
    "Delete",
    "Insert",
    "Home",
    "End",
    "PageUp",
    "PageDown",
    "Comma",
    "Period",
    "Slash",
    "Backslash",
    "BracketLeft",
    "BracketRight",
    "Semicolon",
    "Quote",
    "Backquote",
    "Minus",
    "Equal",
    "IntlBackslash",
];

const MAX_BINDING_LEN: usize = 40;

#[derive(Debug, Clone, Default)]
pub struct HotkeySettings {
    pub bindings: BTreeMap<String, String>,
}

#[derive(Debug, thiserror::Error)]
pub enum HotkeySettingsError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Invalid hotkey settings")]
    Invalid,
    #[error("Hotkey binding is already used by another action")]
    Conflict,
}

pub struct HotkeySettingsPatch {
    pub bindings: BTreeMap<String, Option<String>>,
}

fn valid_key(token: &str) -> bool {
    let bytes = token.as_bytes();
    match bytes.len() {
        1 => bytes[0].is_ascii_uppercase() || bytes[0].is_ascii_digit(),
        2 | 3 if bytes[0] == b'F' => token[1..]
            .parse::<u8>()
            .is_ok_and(|n| (1..=12).contains(&n) && !token[1..].starts_with('0')),
        _ => NAMED_KEYS.contains(&token),
    }
}

pub fn valid_binding(value: &str) -> bool {
    if value.is_empty() || value.len() > MAX_BINDING_LEN {
        return false;
    }
    let parts: Vec<&str> = value.split('+').collect();
    let Some((key, modifiers)) = parts.split_last() else {
        return false;
    };
    let mut last_index = None;
    for modifier in modifiers {
        let Some(index) = MODIFIERS.iter().position(|m| m == modifier) else {
            return false;
        };
        if last_index.is_some_and(|last| index <= last) {
            return false;
        }
        last_index = Some(index);
    }
    valid_key(key)
}

fn parse_bindings(raw: &str) -> HotkeySettings {
    let parsed: BTreeMap<String, serde_json::Value> = serde_json::from_str(raw).unwrap_or_default();
    let bindings = parsed
        .into_iter()
        .filter_map(|(action, value)| match value {
            serde_json::Value::String(binding)
                if known_action(&action) && valid_binding(&binding) =>
            {
                Some((action, binding))
            }
            _ => None,
        })
        .collect();
    HotkeySettings { bindings }
}

pub struct GetHotkeySettings<U: UserRepository> {
    user_repository: Arc<U>,
}

impl<U: UserRepository> GetHotkeySettings<U> {
    pub fn new(user_repository: Arc<U>) -> Self {
        Self { user_repository }
    }

    pub async fn execute(
        &self,
        user_id: UserId,
    ) -> Result<Option<HotkeySettings>, HotkeySettingsError> {
        Ok(self
            .user_repository
            .get_hotkey_settings(&user_id)
            .await?
            .map(|raw| parse_bindings(&raw)))
    }
}

pub struct UpdateHotkeySettings<U: UserRepository> {
    user_repository: Arc<U>,
}

impl<U: UserRepository> UpdateHotkeySettings<U> {
    pub fn new(user_repository: Arc<U>) -> Self {
        Self { user_repository }
    }

    pub async fn execute(
        &self,
        user_id: UserId,
        patch: HotkeySettingsPatch,
    ) -> Result<HotkeySettings, HotkeySettingsError> {
        if patch.bindings.len() > HOTKEY_ACTIONS.len() {
            return Err(HotkeySettingsError::Invalid);
        }
        for (action, binding) in &patch.bindings {
            if !known_action(action) || binding.as_deref().is_some_and(|b| !valid_binding(b)) {
                return Err(HotkeySettingsError::Invalid);
            }
        }
        let patch_json =
            serde_json::to_string(&patch.bindings).map_err(|_| HotkeySettingsError::Invalid)?;
        let raw = self
            .user_repository
            .apply_hotkey_settings_patch(&user_id, &patch_json, &defaults_json())
            .await?
            .ok_or(HotkeySettingsError::Conflict)?;
        Ok(parse_bindings(&raw))
    }
}
