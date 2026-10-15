pub mod repository;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallParticipantStatus {
    Invited,
    Joined,
    Declined,
    Left,
}

impl CallParticipantStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Invited => "invited",
            Self::Joined => "joined",
            Self::Declined => "declined",
            Self::Left => "left",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CallRoster {
    pub ringing: usize,
    pub connected: usize,
}

impl CallRoster {
    pub fn is_abandoned(&self) -> bool {
        self.connected == 0
    }

    pub fn is_over_after_decline(&self) -> bool {
        self.ringing == 0 && self.connected <= 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_call_is_abandoned_once_nobody_is_connected() {
        assert!(CallRoster { ringing: 1, connected: 0 }.is_abandoned());
        assert!(!CallRoster { ringing: 0, connected: 1 }.is_abandoned());
    }

    #[test]
    fn a_decline_ends_the_call_when_the_caller_is_left_alone() {
        assert!(CallRoster { ringing: 0, connected: 1 }.is_over_after_decline());
        assert!(CallRoster { ringing: 0, connected: 0 }.is_over_after_decline());
        assert!(!CallRoster { ringing: 1, connected: 1 }.is_over_after_decline());
        assert!(!CallRoster { ringing: 0, connected: 2 }.is_over_after_decline());
    }
}
