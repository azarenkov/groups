use chrono::Utc;

use super::{FriendshipError, events::FriendshipDomainEvent, pair::FriendshipPair, pair::PairState, status::FriendshipStatus};
use domain::user::id::UserId;

#[test]
fn request_rejects_self_and_creates_pending_rows() {
    let a = UserId::generate();
    let b = UserId::generate();
    assert_eq!(FriendshipPair::request(a, a, Utc::now()).unwrap_err(), FriendshipError::SelfFriendship);
    let mut pair = FriendshipPair::request(a, b, Utc::now()).unwrap();
    assert_eq!(pair.state(), PairState::Pending { sender: a });
    assert_eq!(pair.status_of(&a), Some(FriendshipStatus::PendingOutgoing));
    assert_eq!(pair.status_of(&b), Some(FriendshipStatus::PendingIncoming));
    assert_eq!(pair.rows().len(), 2);
    assert!(matches!(pair.pull_events().as_slice(), [FriendshipDomainEvent::RequestSent(_)]));
}

#[test]
fn resend_is_blocked_while_pending_or_accepted() {
    let a = UserId::generate();
    let b = UserId::generate();
    let mut pair = FriendshipPair::request(a, b, Utc::now()).unwrap();
    assert_eq!(pair.resend(a, Utc::now()).unwrap_err(), FriendshipError::AlreadyRequested);
    assert_eq!(pair.resend(b, Utc::now()).unwrap_err(), FriendshipError::IncomingPending);
    pair.accept(b, Utc::now()).unwrap();
    assert_eq!(pair.resend(a, Utc::now()).unwrap_err(), FriendshipError::AlreadyFriends);
}

#[test]
fn only_the_receiver_can_accept_or_decline() {
    let a = UserId::generate();
    let b = UserId::generate();
    let stranger = UserId::generate();
    let mut pair = FriendshipPair::request(a, b, Utc::now()).unwrap();
    assert_eq!(pair.accept(a, Utc::now()).unwrap_err(), FriendshipError::NoPendingRequest);
    assert_eq!(pair.accept(stranger, Utc::now()).unwrap_err(), FriendshipError::NoPendingRequest);
    pair.pull_events();
    pair.accept(b, Utc::now()).unwrap();
    assert_eq!(pair.state(), PairState::Accepted);
    assert_eq!(pair.accept(b, Utc::now()).unwrap_err(), FriendshipError::NoPendingRequest);
    assert!(matches!(pair.pull_events().as_slice(), [FriendshipDomainEvent::RequestAccepted(_)]));
}

#[test]
fn decline_keeps_only_the_senders_row_and_allows_a_new_request() {
    let a = UserId::generate();
    let b = UserId::generate();
    let mut pair = FriendshipPair::request(a, b, Utc::now()).unwrap();
    pair.decline(b, Utc::now()).unwrap();
    assert_eq!(pair.state(), PairState::Declined { sender: a });
    assert_eq!(pair.status_of(&a), Some(FriendshipStatus::Declined));
    assert_eq!(pair.status_of(&b), None);
    assert_eq!(pair.rows().len(), 1);
    pair.resend(b, Utc::now()).unwrap();
    assert_eq!(pair.state(), PairState::Pending { sender: b });
}

#[test]
fn remove_declines_incoming_requests_and_deletes_everything_else() {
    let a = UserId::generate();
    let b = UserId::generate();
    let mut pair = FriendshipPair::request(a, b, Utc::now()).unwrap();
    pair.remove(b, Utc::now()).unwrap();
    assert_eq!(pair.state(), PairState::Declined { sender: a });

    let mut pair = FriendshipPair::request(a, b, Utc::now()).unwrap();
    pair.remove(a, Utc::now()).unwrap();
    assert_eq!(pair.state(), PairState::Removed);
    assert!(pair.rows().is_empty());
    assert_eq!(pair.remove(a, Utc::now()).unwrap_err(), FriendshipError::NotFriends);
    assert_eq!(pair.remove(UserId::generate(), Utc::now()).unwrap_err(), FriendshipError::NotFriends);
}
