pub mod repository;

use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MediaRevision {
    pub source: Uuid,
    pub seq: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MediaState {
    mic_muted: bool,
    deafened: bool,
}

impl MediaState {
    pub fn new(mic_muted: bool, deafened: bool) -> Self {
        Self { mic_muted, deafened }
    }

    pub fn mic_muted(&self) -> bool {
        self.mic_muted
    }

    pub fn deafened(&self) -> bool {
        self.deafened
    }

    pub fn mic_live(&self) -> bool {
        !self.mic_muted && !self.deafened
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deafening_silences_the_microphone_but_keeps_its_own_toggle() {
        let state = MediaState::new(false, true);
        assert!(!state.mic_live());
        assert!(!state.mic_muted());
        assert!(state.deafened());
    }

    #[test]
    fn the_microphone_can_be_muted_without_deafening() {
        let state = MediaState::new(true, false);
        assert!(state.mic_muted());
        assert!(!state.deafened());
        assert_eq!(MediaState::default(), MediaState::new(false, false));
    }
}
