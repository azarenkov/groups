use chrono::Utc;
use contracts::events::user::{UserLifecycleV1, UserRenamedV1, VerificationIssuedV1};
use domain::{user::email::Email, user::id::UserId, user::locale::Locale, user::name::UserName};
use identity_domain::{events::IdentityEvent, user::events::UserEvent, user::events::lifecycle::UserRegistered, user::events::lifecycle::UserRenamed, verification::channel::VerificationChannel, verification::code::VerificationCode, verification::code_hash::CodeHash, verification::purpose::VerificationPurpose};
use identity_infrastructure::outbox::mapping::to_record;
use ports::outbox::record::AggregateType;

fn decode<T: serde::de::DeserializeOwned>(value: serde_json::Value) -> T {
    serde_json::from_value(value).expect("payload must match contract")
}

#[test]
fn registered_exposes_name_for_projections() {
    let user_id = UserId::generate();
    let event = UserEvent::Registered(UserRegistered::record(
        user_id,
        UserName::new("tester".to_string()),
        Email::new("t@example.com".to_string()),
        None,
        None,
        Utc::now(),
    ));
    let record = to_record(&IdentityEvent::from(event), Some("req")).unwrap();
    assert_eq!(record.event_type, "registered");
    assert_eq!(record.aggregate_type, AggregateType::User);
    assert_eq!(record.request_id.as_deref(), Some("req"));
    let evt: UserLifecycleV1 = decode(record.payload);
    assert_eq!(evt.user_id, user_id.uuid());
    assert_eq!(evt.name.as_deref(), Some("tester"));
}

#[test]
fn renamed_maps_to_user_renamed() {
    let user_id = UserId::generate();
    let now = Utc::now();
    let event = UserEvent::Renamed(UserRenamed::record(user_id, UserName::new("renamed".to_string()), now));
    let record = to_record(&IdentityEvent::from(event), None).unwrap();
    assert_eq!(record.event_type, "renamed");
    let evt: UserRenamedV1 = decode(record.payload);
    assert_eq!(evt.name, "renamed");
    assert_eq!(evt.occurred_at, now);
}

#[test]
fn verification_issued_carries_code_email_and_locale() {
    let user_id = UserId::generate();
    let mut code = VerificationCode::issue(
        user_id,
        Email::new("t@example.com".to_string()),
        "123456".to_string(),
        VerificationChannel::Email,
        CodeHash::new("hash".to_string()),
        VerificationPurpose::Registration,
        Locale::default(),
    );
    let event = code.pull_events().remove(0);
    let record = to_record(&IdentityEvent::from(event), None).unwrap();
    assert_eq!(record.event_type, "verification.issued");
    let evt: VerificationIssuedV1 = decode(record.payload);
    assert_eq!(evt.code, "123456");
    assert_eq!(evt.email, "t@example.com");
    assert_eq!(evt.channel, "email");
    assert_eq!(evt.purpose, "registration");
    assert_eq!(evt.locale.as_deref(), Some(Locale::default().as_str()));
}
