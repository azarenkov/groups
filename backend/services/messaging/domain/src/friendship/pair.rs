use chrono::{DateTime, Utc};

use crate::{friendship::Friendship, friendship::FriendshipError, friendship::events::FriendshipDomainEvent, friendship::events::lifecycle::FriendRequestAccepted, friendship::events::lifecycle::FriendRequestSent, friendship::events::lifecycle::FriendshipRemoved, friendship::status::FriendshipStatus};
use domain::user::id::UserId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairState {
    Pending { sender: UserId },
    Accepted,
    Declined { sender: UserId },
    Removed,
}

#[derive(Debug)]
pub struct FriendshipPair {
    a: UserId,
    b: UserId,
    state: PairState,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    events: Vec<FriendshipDomainEvent>,
}

impl FriendshipPair {
    pub fn request(sender: UserId, receiver: UserId, now: DateTime<Utc>) -> Result<Self, FriendshipError> {
        if sender == receiver {
            return Err(FriendshipError::SelfFriendship);
        }
        let mut pair = Self {
            a: sender,
            b: receiver,
            state: PairState::Pending { sender },
            created_at: now,
            updated_at: now,
            events: Vec::new(),
        };
        pair.record(FriendshipDomainEvent::RequestSent(FriendRequestSent::record(sender, receiver, now)));
        Ok(pair)
    }

    pub fn restore(a: UserId, b: UserId, state: PairState, created_at: DateTime<Utc>, updated_at: DateTime<Utc>) -> Self {
        Self {
            a,
            b,
            state,
            created_at,
            updated_at,
            events: Vec::new(),
        }
    }

    pub fn from_rows(a_to_b: Option<&Friendship>, b_to_a: Option<&Friendship>) -> Option<Self> {
        let (row, mirror) = match (a_to_b, b_to_a) {
            (Some(row), mirror) => (row, mirror),
            (None, Some(row)) => (row, None),
            (None, None) => return None,
        };
        let (a, b) = (row.user_id, row.friend_id);
        let state = match row.status {
            FriendshipStatus::Accepted => PairState::Accepted,
            FriendshipStatus::PendingOutgoing => PairState::Pending { sender: a },
            FriendshipStatus::PendingIncoming => PairState::Pending { sender: b },
            FriendshipStatus::Declined => PairState::Declined { sender: a },
        };
        let created_at = mirror.map_or(row.created_at, |m| row.created_at.min(m.created_at));
        let updated_at = mirror.map_or(row.updated_at, |m| row.updated_at.max(m.updated_at));
        Some(Self::restore(a, b, state, created_at, updated_at))
    }

    pub fn resend(&mut self, sender: UserId, now: DateTime<Utc>) -> Result<(), FriendshipError> {
        let receiver = self.other(&sender).ok_or(FriendshipError::NotFriends)?;
        match self.state {
            PairState::Accepted => return Err(FriendshipError::AlreadyFriends),
            PairState::Pending { sender: s } if s == sender => return Err(FriendshipError::AlreadyRequested),
            PairState::Pending { .. } => return Err(FriendshipError::IncomingPending),
            PairState::Declined { .. } | PairState::Removed => {}
        }
        self.state = PairState::Pending { sender };
        self.updated_at = now;
        self.record(FriendshipDomainEvent::RequestSent(FriendRequestSent::record(sender, receiver, now)));
        Ok(())
    }

    pub fn accept(&mut self, by: UserId, now: DateTime<Utc>) -> Result<(), FriendshipError> {
        let sender = self.incoming_sender_for(&by)?;
        self.state = PairState::Accepted;
        self.updated_at = now;
        self.record(FriendshipDomainEvent::RequestAccepted(FriendRequestAccepted::record(by, sender, now)));
        Ok(())
    }

    pub fn decline(&mut self, by: UserId, now: DateTime<Utc>) -> Result<(), FriendshipError> {
        let sender = self.incoming_sender_for(&by)?;
        self.state = PairState::Declined { sender };
        self.updated_at = now;
        self.record(FriendshipDomainEvent::Removed(FriendshipRemoved::record(by, sender, now)));
        Ok(())
    }

    pub fn remove(&mut self, by: UserId, now: DateTime<Utc>) -> Result<(), FriendshipError> {
        let other = self.other(&by).ok_or(FriendshipError::NotFriends)?;
        match self.state {
            PairState::Removed => return Err(FriendshipError::NotFriends),
            PairState::Pending { sender } if sender != by => return self.decline(by, now),
            _ => {}
        }
        self.state = PairState::Removed;
        self.updated_at = now;
        self.record(FriendshipDomainEvent::Removed(FriendshipRemoved::record(by, other, now)));
        Ok(())
    }

    fn incoming_sender_for(&self, by: &UserId) -> Result<UserId, FriendshipError> {
        if self.other(by).is_none() {
            return Err(FriendshipError::NoPendingRequest);
        }
        match self.state {
            PairState::Pending { sender } if sender != *by => Ok(sender),
            _ => Err(FriendshipError::NoPendingRequest),
        }
    }

    fn other(&self, user: &UserId) -> Option<UserId> {
        if *user == self.a {
            Some(self.b)
        } else if *user == self.b {
            Some(self.a)
        } else {
            None
        }
    }

    fn record(&mut self, event: FriendshipDomainEvent) {
        self.events.push(event);
    }

    pub fn pull_events(&mut self) -> Vec<FriendshipDomainEvent> {
        std::mem::take(&mut self.events)
    }

    pub fn users(&self) -> (UserId, UserId) {
        (self.a, self.b)
    }

    pub fn state(&self) -> PairState {
        self.state
    }

    pub fn status_of(&self, user: &UserId) -> Option<FriendshipStatus> {
        self.other(user)?;
        match self.state {
            PairState::Accepted => Some(FriendshipStatus::Accepted),
            PairState::Pending { sender } if sender == *user => Some(FriendshipStatus::PendingOutgoing),
            PairState::Pending { .. } => Some(FriendshipStatus::PendingIncoming),
            PairState::Declined { sender } if sender == *user => Some(FriendshipStatus::Declined),
            PairState::Declined { .. } | PairState::Removed => None,
        }
    }

    pub fn rows(&self) -> Vec<Friendship> {
        [self.a, self.b]
            .into_iter()
            .filter_map(|user| {
                let friend = self.other(&user)?;
                let status = self.status_of(&user)?;
                Some(Friendship {
                    user_id: user,
                    friend_id: friend,
                    status,
                    created_at: self.created_at,
                    updated_at: self.updated_at,
                })
            })
            .collect()
    }
}
