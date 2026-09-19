use std::collections::BTreeMap;

use super::appearance::{AppearanceError, AppearancePatch, AppearanceSettings, Theme};
use super::hotkeys::{Binding, HotkeyAction, HotkeyBindings, HotkeyError, HotkeyPatch, HotkeyScope};
use super::notification::{
    MentionLevel, NotificationSettings, NotificationSettingsError, NotificationSettingsPatch,
};
use super::voice::{NoiseMode, VoiceSettings, VoiceSettingsError, VoiceSettingsPatch};

fn hotkey_patch(entries: &[(&str, Option<&str>)]) -> HotkeyPatch {
    HotkeyPatch {
        bindings: entries
            .iter()
            .map(|(id, value)| (id.to_string(), value.map(str::to_string)))
            .collect(),
    }
}

#[test]
fn binding_grammar_accepts_canonical_chords() {
    for chord in ["Mod+K", "Alt+ArrowUp", "Mod+Shift+M", "F5", "Ctrl+Alt+Shift+Comma", "0"] {
        assert!(Binding::parse(chord).is_ok(), "{chord}");
    }
}

#[test]
fn binding_grammar_rejects_malformed_chords() {
    for chord in [
        "",
        "Shift+Mod+K",
        "Mod+Mod+K",
        "Mod+",
        "+K",
        "Mod+k",
        "F0",
        "F13",
        "F01",
        "Meta+K",
        "Mod+Numpad1",
    ] {
        assert_eq!(Binding::parse(chord), Err(HotkeyError::InvalidBinding), "{chord}");
    }
}

#[test]
fn global_scope_requires_strong_modifier_or_function_key() {
    let ok = |c: &str| Binding::parse(c).unwrap().allowed_for(HotkeyScope::Global);
    assert!(ok("Mod+L"));
    assert!(ok("Alt+ArrowDown"));
    assert!(ok("F1"));
    assert!(ok("Shift+Escape"));
    assert!(!ok("L"));
    assert!(!ok("Shift+L"));
    assert!(!ok("ArrowUp"));
}

#[test]
fn composer_scope_allows_bare_navigation_keys() {
    let ok = |c: &str| Binding::parse(c).unwrap().allowed_for(HotkeyScope::Composer);
    assert!(ok("ArrowUp"));
    assert!(ok("PageUp"));
    assert!(ok("Mod+ArrowUp"));
    assert!(!ok("L"));
    assert!(!ok("Shift+Enter"));
}

#[test]
fn apply_sets_and_resets_overrides() {
    let bindings = HotkeyBindings::default();
    let next = bindings
        .apply(&hotkey_patch(&[("toggleMute", Some("Mod+Shift+X"))]))
        .unwrap();
    assert_eq!(next.effective(HotkeyAction::ToggleMute).unwrap().as_str(), "Mod+Shift+X");
    assert_eq!(next.overrides().len(), 1);
    let reset = next.apply(&hotkey_patch(&[("toggleMute", None)])).unwrap();
    assert!(reset.overrides().is_empty());
    assert_eq!(reset.effective(HotkeyAction::ToggleMute).unwrap().as_str(), "Mod+Shift+M");
}

#[test]
fn apply_drops_override_equal_to_default() {
    let next = HotkeyBindings::default()
        .apply(&hotkey_patch(&[("quickSwitcher", Some("Mod+K"))]))
        .unwrap();
    assert!(next.overrides().is_empty());
}

#[test]
fn apply_rejects_unknown_action_and_disallowed_scope() {
    let bindings = HotkeyBindings::default();
    assert_eq!(
        bindings.apply(&hotkey_patch(&[("reactToLast", Some("Mod+E"))])),
        Err(HotkeyError::UnknownAction)
    );
    assert_eq!(
        bindings.apply(&hotkey_patch(&[("leaveVoice", Some("L"))])),
        Err(HotkeyError::NotAllowed)
    );
}

#[test]
fn apply_rejects_chord_held_by_another_action_including_defaults() {
    let bindings = HotkeyBindings::default();
    assert_eq!(
        bindings.apply(&hotkey_patch(&[("leaveVoice", Some("Mod+K"))])),
        Err(HotkeyError::Conflict(HotkeyAction::QuickSwitcher))
    );
    assert_eq!(
        bindings.apply(&hotkey_patch(&[
            ("prevChannel", Some("Mod+Q")),
            ("nextChannel", Some("Mod+Q")),
        ])),
        Err(HotkeyError::Conflict(HotkeyAction::PrevChannel))
    );
}

#[test]
fn apply_allows_reusing_a_freed_chord() {
    let moved = HotkeyBindings::default()
        .apply(&hotkey_patch(&[("quickSwitcher", Some("Mod+P"))]))
        .unwrap();
    let reused = moved
        .apply(&hotkey_patch(&[("leaveVoice", Some("Mod+K"))]))
        .unwrap();
    assert_eq!(reused.effective(HotkeyAction::LeaveVoice).unwrap().as_str(), "Mod+K");
    assert_eq!(reused.effective(HotkeyAction::QuickSwitcher).unwrap().as_str(), "Mod+P");
}

#[test]
fn override_claiming_a_new_default_unbinds_that_default_until_reassigned() {
    let stored: BTreeMap<String, String> = [("historyBack", "Mod+Shift+O")]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    let restored = HotkeyBindings::restore(stored);
    assert_eq!(restored.effective(HotkeyAction::ToggleCallView), None);
    assert_eq!(restored.effective(HotkeyAction::HistoryBack).unwrap().as_str(), "Mod+Shift+O");
    let unrelated = restored
        .apply(&hotkey_patch(&[("quickSwitcher", Some("Mod+P"))]))
        .unwrap();
    assert_eq!(unrelated.effective(HotkeyAction::ToggleCallView), None);
    assert_eq!(
        restored.apply(&hotkey_patch(&[("toggleCallView", Some("Mod+Shift+O"))])),
        Err(HotkeyError::Conflict(HotkeyAction::HistoryBack))
    );
    let reassigned = restored
        .apply(&hotkey_patch(&[("toggleCallView", Some("Mod+Shift+V"))]))
        .unwrap();
    assert_eq!(reassigned.effective(HotkeyAction::ToggleCallView).unwrap().as_str(), "Mod+Shift+V");
    let released = restored.apply(&hotkey_patch(&[("historyBack", None)])).unwrap();
    assert_eq!(released.effective(HotkeyAction::ToggleCallView).unwrap().as_str(), "Mod+Shift+O");

    let legacy: BTreeMap<String, String> = [("leaveVoice", "Mod+Shift+D")]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    let legacy = HotkeyBindings::restore(legacy);
    assert_eq!(legacy.effective(HotkeyAction::ToggleDeafen), None);
    assert_eq!(legacy.effective(HotkeyAction::LeaveVoice).unwrap().as_str(), "Mod+Shift+D");
    assert!(legacy.apply(&hotkey_patch(&[("openSettings", Some("Mod+Shift+P"))])).is_ok());
}

#[test]
fn restore_drops_unknown_actions_and_invalid_chords() {
    let stored: BTreeMap<String, String> = [
        ("toggleMute", "Mod+Shift+X"),
        ("reactToLast", "Mod+E"),
        ("leaveVoice", "Shift+Mod+L"),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), v.to_string()))
    .collect();
    let restored = HotkeyBindings::restore(stored);
    assert_eq!(restored.overrides().len(), 1);
    assert_eq!(restored.effective(HotkeyAction::ToggleMute).unwrap().as_str(), "Mod+Shift+X");
}

#[test]
fn voice_settings_validate_ranges_modes_and_ptt_key() {
    let base = VoiceSettings::default();
    assert_eq!(base.ptt_key().as_str(), "KeyM");
    let next = base
        .apply(&VoiceSettingsPatch {
            mic_volume: Some(30),
            noise_mode: Some("high".to_string()),
            ptt_key: Some("F4".to_string()),
            ..VoiceSettingsPatch::default()
        })
        .unwrap();
    assert_eq!(next.mic_volume(), 30);
    assert_eq!(next.noise_mode(), NoiseMode::High);
    assert_eq!(next.ptt_key().as_str(), "F4");
    assert_eq!(
        base.apply(&VoiceSettingsPatch { output_volume: Some(101), ..VoiceSettingsPatch::default() }),
        Err(VoiceSettingsError::InvalidVolume)
    );
    assert_eq!(
        base.apply(&VoiceSettingsPatch { sensitivity: Some(-1), ..VoiceSettingsPatch::default() }),
        Err(VoiceSettingsError::InvalidSensitivity)
    );
    assert_eq!(
        base.apply(&VoiceSettingsPatch { noise_mode: Some("loud".to_string()), ..VoiceSettingsPatch::default() }),
        Err(VoiceSettingsError::InvalidNoiseMode)
    );
    for key in ["", "ShiftLeft", "MetaRight", "Key-M", "a".repeat(33).as_str()] {
        assert_eq!(
            base.apply(&VoiceSettingsPatch { ptt_key: Some(key.to_string()), ..VoiceSettingsPatch::default() }),
            Err(VoiceSettingsError::InvalidPttKey),
            "{key}"
        );
    }
}

#[test]
fn voice_settings_restore_keeps_legacy_modifier_ptt_key() {
    let restored = VoiceSettings::restore(100, 100, "standard", true, 50, true, "ShiftLeft".to_string(), "720p60");
    assert_eq!(restored.ptt_key().as_str(), "ShiftLeft");
    assert_eq!(VoiceSettings::restore(1, 2, "weird", false, 3, false, "KeyA".to_string(), "weird").noise_mode(), NoiseMode::Standard);
}

#[test]
fn notification_settings_validate_mentions_and_time() {
    let base = NotificationSettings::default();
    let next = base
        .apply(&NotificationSettingsPatch {
            mentions: Some("none".to_string()),
            quiet_start: Some("23:30".to_string()),
            quiet_hours_enabled: Some(true),
            ..NotificationSettingsPatch::default()
        })
        .unwrap();
    assert_eq!(next.mentions(), MentionLevel::None);
    assert_eq!(next.quiet_start().as_str(), "23:30");
    assert!(next.quiet_hours_enabled());
    assert_eq!(
        base.apply(&NotificationSettingsPatch { mentions: Some("loud".to_string()), ..NotificationSettingsPatch::default() }),
        Err(NotificationSettingsError::InvalidMentions)
    );
    for time in ["24:00", "23:60", "7:00", "0700", "ab:cd"] {
        assert_eq!(
            base.apply(&NotificationSettingsPatch { quiet_end: Some(time.to_string()), ..NotificationSettingsPatch::default() }),
            Err(NotificationSettingsError::InvalidTime),
            "{time}"
        );
    }
}

#[test]
fn appearance_settings_validate_theme_accent_and_font_size() {
    let base = AppearanceSettings::default();
    assert_eq!(base.theme(), Theme::System);
    let next = base
        .apply(&AppearancePatch {
            theme: Some("dark".to_string()),
            accent: Some("#0071e3".to_string()),
            chat_font_size: Some(18),
            reduce_motion: Some(true),
        })
        .unwrap();
    assert_eq!(next.theme(), Theme::Dark);
    assert_eq!(next.accent(), "#0071e3");
    assert_eq!(next.chat_font_size(), 18);
    assert!(next.reduce_motion());
    assert_eq!(
        base.apply(&AppearancePatch { theme: Some("blue".to_string()), ..AppearancePatch::default() }),
        Err(AppearanceError::InvalidTheme)
    );
    assert_eq!(
        base.apply(&AppearancePatch { accent: Some("0071e3".to_string()), ..AppearancePatch::default() }),
        Err(AppearanceError::InvalidAccent)
    );
    assert_eq!(
        base.apply(&AppearancePatch { chat_font_size: Some(11), ..AppearancePatch::default() }),
        Err(AppearanceError::InvalidFontSize)
    );
    assert!(base.apply(&AppearancePatch { accent: Some(String::new()), ..AppearancePatch::default() }).is_ok());
}

#[test]
fn a_plan_decides_which_stream_quality_and_noise_mode_are_open() {
    use crate::user::settings::voice::{StreamQuality, VoiceSettingsPatch};
    use crate::user::tier::Tier;

    let wants_4k = VoiceSettings::default()
        .apply(&VoiceSettingsPatch {
            stream_quality: Some("2160p60".to_string()),
            noise_mode: Some("high".to_string()),
            ..Default::default()
        })
        .unwrap();

    assert!(!wants_4k.allowed_for(Tier::Free));
    assert!(!wants_4k.allowed_for(Tier::Pro));
    assert!(wants_4k.allowed_for(Tier::Premium));

    let as_free = wants_4k.clone().limited_to(Tier::Free);
    assert_eq!(as_free.stream_quality(), StreamQuality::P720);
    assert_eq!(as_free.noise_mode(), NoiseMode::Standard);

    let as_pro = wants_4k.clone().limited_to(Tier::Pro);
    assert_eq!(as_pro.stream_quality(), StreamQuality::P1440);
    assert_eq!(as_pro.noise_mode(), NoiseMode::High);

    assert_eq!(wants_4k.limited_to(Tier::Premium).stream_quality(), StreamQuality::P2160);
    assert_eq!(StreamQuality::parse("8k"), None);
}
