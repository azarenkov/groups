use contracts::events::user::{UserActivityChangedV1, UserEntitlementsChangedV1, UserPresencePreferenceChangedV1, UserTrialEndingV1, UserLifecycleV1, UserProfileChangedV1, UserRenamedV1, VerificationIssuedV1};
use identity_domain::{events::IdentityEvent, user::events::UserEvent, verification::events::VerificationEvent};
use ports::{outbox::record::AggregateType, outbox::record::OutboxRecord, repository::RepositoryError};
use uuid::Uuid;
use infrastructure_common::outbox::encoding::{encode, snake_case};

pub fn to_record(event: &IdentityEvent, request_id: Option<&str>) -> Result<OutboxRecord, RepositoryError> {
    let (aggregate_id, event_type, occurred_at, payload) = match event {
        IdentityEvent::User(e) => (
            e.user_id().uuid(),
            e.event_type(),
            e.occurred_at(),
            user_payload(e)?,
        ),
        IdentityEvent::Verification(e) => (
            e.user_id().uuid(),
            e.event_type(),
            e.occurred_at(),
            verification_payload(e)?,
        ),
    };
    Ok(OutboxRecord {
        id: Uuid::new_v4(),
        aggregate_id,
        aggregate_type: AggregateType::User,
        parent_id: None,
        parent_type: None,
        event_type: event_type.to_string(),
        payload,
        occurred_at,
        request_id: request_id.map(str::to_string),
    })
}

fn user_payload(event: &UserEvent) -> Result<serde_json::Value, RepositoryError> {
    match event {
        UserEvent::Registered(e) => encode(&UserLifecycleV1 {
            user_id: e.user_id.uuid(),
            occurred_at: e.occurred_at,
            name: Some(e.name.as_str().to_string()),
        }),
        UserEvent::EmailVerified(e) => encode(&UserLifecycleV1 {
            user_id: e.user_id.uuid(),
            occurred_at: e.occurred_at,
            name: None,
        }),
        UserEvent::PhoneVerified(e) => encode(&UserLifecycleV1 {
            user_id: e.user_id.uuid(),
            occurred_at: e.occurred_at,
            name: None,
        }),
        UserEvent::LoggedIn(e) => encode(&UserLifecycleV1 {
            user_id: e.user_id.uuid(),
            occurred_at: e.occurred_at,
            name: None,
        }),
        UserEvent::LoggedOut(e) => encode(&UserLifecycleV1 {
            user_id: e.user_id.uuid(),
            occurred_at: e.occurred_at,
            name: None,
        }),
        UserEvent::Renamed(e) => encode(&UserRenamedV1 {
            user_id: e.user_id.uuid(),
            name: e.name.as_str().to_string(),
            occurred_at: e.occurred_at,
        }),
        UserEvent::PasswordChanged(e) => encode(&UserLifecycleV1 {
            user_id: e.user_id.uuid(),
            occurred_at: e.occurred_at,
            name: None,
        }),
        UserEvent::EmailChanged(e) => encode(&UserLifecycleV1 {
            user_id: e.user_id.uuid(),
            occurred_at: e.occurred_at,
            name: None,
        }),
        UserEvent::Disabled(e) => encode(&UserLifecycleV1 {
            user_id: e.user_id.uuid(),
            occurred_at: e.occurred_at,
            name: None,
        }),
        UserEvent::Reactivated(e) => encode(&UserLifecycleV1 {
            user_id: e.user_id.uuid(),
            occurred_at: e.occurred_at,
            name: None,
        }),
        UserEvent::Deleted(e) => encode(&UserLifecycleV1 {
            user_id: e.user_id.uuid(),
            occurred_at: e.occurred_at,
            name: None,
        }),
        UserEvent::ProfileChanged(e) => encode(&UserProfileChangedV1 {
            badge_style: e.badge_style.clone(),
            user_id: e.user_id.uuid(),
            bio: e.bio.clone(),
            show_activity: e.show_activity,
            show_game_activity: e.show_game_activity,
            active: e.active,
            timezone: e.timezone.clone(),
            emoji_status_sticker_id: e.emoji_status.map(|status| status.sticker_id()),
            emoji_status_until: e.emoji_status.and_then(|status| status.until()),
            emoji_status_included: true,
            occurred_at: e.occurred_at,
        }),
        UserEvent::ActivityChanged(e) => encode(&UserActivityChangedV1 {
            user_id: e.user_id.uuid(),
            game_id: e.game_id.clone(),
            name: e.name.clone(),
            started_at: e.started_at,
            occurred_at: e.occurred_at,
        }),
        UserEvent::PresencePreferenceChanged(e) => encode(&UserPresencePreferenceChangedV1 {
            user_id: e.user_id.uuid(),
            appear_offline: e.appear_offline,
            occurred_at: e.occurred_at,
        }),
        UserEvent::EntitlementsChanged(e) => encode(&UserEntitlementsChangedV1 {
            user_id: e.user_id.uuid(),
            tier: e.tier.clone(),
            valid_until: e.valid_until,
            member_number: e.member_number,
            trial: e.trial,
            occurred_at: e.occurred_at,
        }),
        UserEvent::TrialEnding(e) => encode(&UserTrialEndingV1 {
            user_id: e.user_id.uuid(),
            email: e.email.as_str().to_string(),
            locale: Some(e.locale.clone()),
            ends_at: e.ends_at,
            occurred_at: e.occurred_at,
        }),
    }
}

fn verification_payload(event: &VerificationEvent) -> Result<serde_json::Value, RepositoryError> {
    match event {
        VerificationEvent::Issued(e) => encode(&VerificationIssuedV1 {
            user_id: e.user_id.uuid(),
            verification_code_id: e.verification_code_id.uuid(),
            channel: snake_case(&e.channel)?,
            purpose: snake_case(&e.purpose)?,
            email: e.email.as_str().to_string(),
            code: e.code.clone(),
            occurred_at: e.occurred_at,
            locale: Some(e.locale.as_str().to_string()),
        }),
    }
}

