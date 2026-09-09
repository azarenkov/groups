use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum HotkeyAction {
    QuickSwitcher,
    PrevChannel,
    NextChannel,
    HistoryBack,
    HistoryForward,
    OpenSettings,
    EditLastMessage,
    ToggleMute,
    LeaveVoice,
    ToggleCallView,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotkeyScope {
    Global,
    Composer,
}

impl HotkeyAction {
    pub const ALL: [HotkeyAction; 10] = [
        Self::QuickSwitcher,
        Self::PrevChannel,
        Self::NextChannel,
        Self::HistoryBack,
        Self::HistoryForward,
        Self::OpenSettings,
        Self::EditLastMessage,
        Self::ToggleMute,
        Self::LeaveVoice,
        Self::ToggleCallView,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Self::QuickSwitcher => "quickSwitcher",
            Self::PrevChannel => "prevChannel",
            Self::NextChannel => "nextChannel",
            Self::HistoryBack => "historyBack",
            Self::HistoryForward => "historyForward",
            Self::OpenSettings => "openSettings",
            Self::EditLastMessage => "editLastMessage",
            Self::ToggleMute => "toggleMute",
            Self::LeaveVoice => "leaveVoice",
            Self::ToggleCallView => "toggleCallView",
        }
    }

    pub fn parse(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|action| action.id() == id)
    }

    pub fn scope(self) -> HotkeyScope {
        match self {
            Self::EditLastMessage => HotkeyScope::Composer,
            _ => HotkeyScope::Global,
        }
    }

    pub fn default_binding(self) -> Binding {
        Binding(
            match self {
                Self::QuickSwitcher => "Mod+K",
                Self::PrevChannel => "Alt+ArrowUp",
                Self::NextChannel => "Alt+ArrowDown",
                Self::HistoryBack => "Mod+BracketLeft",
                Self::HistoryForward => "Mod+BracketRight",
                Self::OpenSettings => "Mod+Comma",
                Self::EditLastMessage => "ArrowUp",
                Self::ToggleMute => "Mod+Shift+M",
                Self::LeaveVoice => "Mod+Shift+L",
                Self::ToggleCallView => "Mod+Shift+O",
            }
            .to_string(),
        )
    }
}

const MODIFIERS: &[&str] = &["Mod", "Ctrl", "Alt", "Shift"];
const STRONG_MODIFIERS: &[&str] = &["Mod", "Ctrl", "Alt"];

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

const NAV_KEYS: &[&str] = &[
    "ArrowUp",
    "ArrowDown",
    "ArrowLeft",
    "ArrowRight",
    "Home",
    "End",
    "PageUp",
    "PageDown",
    "Escape",
];

const MAX_BINDING_LEN: usize = 40;

fn is_function_key(key: &str) -> bool {
    key.len() >= 2
        && key.len() <= 3
        && key.starts_with('F')
        && !key[1..].starts_with('0')
        && key[1..].parse::<u8>().is_ok_and(|n| (1..=12).contains(&n))
}

fn valid_key(token: &str) -> bool {
    let bytes = token.as_bytes();
    (bytes.len() == 1 && (bytes[0].is_ascii_uppercase() || bytes[0].is_ascii_digit()))
        || is_function_key(token)
        || NAMED_KEYS.contains(&token)
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Binding(String);

impl Binding {
    pub fn parse(value: &str) -> Result<Self, HotkeyError> {
        if value.is_empty() || value.len() > MAX_BINDING_LEN {
            return Err(HotkeyError::InvalidBinding);
        }
        let parts: Vec<&str> = value.split('+').collect();
        let Some((key, modifiers)) = parts.split_last() else {
            return Err(HotkeyError::InvalidBinding);
        };
        let mut last_index = None;
        for modifier in modifiers {
            let Some(index) = MODIFIERS.iter().position(|m| m == modifier) else {
                return Err(HotkeyError::InvalidBinding);
            };
            if last_index.is_some_and(|last| index <= last) {
                return Err(HotkeyError::InvalidBinding);
            }
            last_index = Some(index);
        }
        if !valid_key(key) {
            return Err(HotkeyError::InvalidBinding);
        }
        Ok(Self(value.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    fn key(&self) -> &str {
        self.0.rsplit('+').next().unwrap_or("")
    }

    fn has_strong_modifier(&self) -> bool {
        self.0
            .split('+')
            .take(self.0.matches('+').count())
            .any(|m| STRONG_MODIFIERS.contains(&m))
    }

    pub fn allowed_for(&self, scope: HotkeyScope) -> bool {
        if self.has_strong_modifier() {
            return true;
        }
        let key = self.key();
        match scope {
            HotkeyScope::Global => is_function_key(key) || key == "Escape",
            HotkeyScope::Composer => is_function_key(key) || NAV_KEYS.contains(&key),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum HotkeyError {
    #[error("Unknown hotkey action")]
    UnknownAction,
    #[error("Invalid hotkey binding")]
    InvalidBinding,
    #[error("Binding is not allowed for this action")]
    NotAllowed,
    #[error("Binding is already used by {}", .0.id())]
    Conflict(HotkeyAction),
}

#[derive(Debug, Clone, Default)]
pub struct HotkeyPatch {
    pub bindings: BTreeMap<String, Option<String>>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HotkeyBindings {
    overrides: BTreeMap<HotkeyAction, Binding>,
}

impl HotkeyBindings {
    pub fn restore(stored: BTreeMap<String, String>) -> Self {
        let overrides = stored
            .into_iter()
            .filter_map(|(id, value)| {
                let action = HotkeyAction::parse(&id)?;
                let binding = Binding::parse(&value).ok()?;
                Some((action, binding))
            })
            .collect();
        Self { overrides }
    }

    pub fn overrides(&self) -> &BTreeMap<HotkeyAction, Binding> {
        &self.overrides
    }

    pub fn effective(&self, action: HotkeyAction) -> Option<Binding> {
        if let Some(binding) = self.overrides.get(&action) {
            return Some(binding.clone());
        }
        let default = action.default_binding();
        let claimed = self
            .overrides
            .iter()
            .any(|(other, binding)| *other != action && *binding == default);
        (!claimed).then_some(default)
    }

    fn requested(&self, action: HotkeyAction) -> Binding {
        self.overrides
            .get(&action)
            .cloned()
            .unwrap_or_else(|| action.default_binding())
    }

    pub fn apply(&self, patch: &HotkeyPatch) -> Result<Self, HotkeyError> {
        let mut next = self.clone();
        let mut touched = Vec::new();
        for (id, value) in &patch.bindings {
            let action = HotkeyAction::parse(id).ok_or(HotkeyError::UnknownAction)?;
            touched.push(action);
            match value.as_deref() {
                None => {
                    next.overrides.remove(&action);
                }
                Some(raw) => {
                    let binding = Binding::parse(raw)?;
                    if !binding.allowed_for(action.scope()) {
                        return Err(HotkeyError::NotAllowed);
                    }
                    if binding == action.default_binding() {
                        next.overrides.remove(&action);
                    } else {
                        next.overrides.insert(action, binding);
                    }
                }
            }
        }
        let mut seen: BTreeMap<Binding, HotkeyAction> = BTreeMap::new();
        for action in HotkeyAction::ALL {
            if let Some(previous) = seen.insert(next.requested(action), action) {
                let previous_touched = touched.contains(&previous);
                let action_touched = touched.contains(&action);
                if !previous_touched && !action_touched {
                    continue;
                }
                let holder = if previous_touched && !action_touched {
                    action
                } else {
                    previous
                };
                return Err(HotkeyError::Conflict(holder));
            }
        }
        Ok(next)
    }
}
