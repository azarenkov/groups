use chrono::Utc;
use domain::user::id::UserId;
use uuid::Uuid;

use super::super::report::{
    MAX_DETAILS_CHARS, Report, ReportContext, ReportError, ReportReason, ReportTarget,
};

fn report_user(reason: ReportReason, details: Option<&str>) -> Result<Report, ReportError> {
    Report::create(
        UserId::generate(),
        ReportTarget::User,
        Uuid::new_v4(),
        "offender".to_string(),
        reason,
        details.map(|d| d.to_string()),
        ReportContext::default(),
        Utc::now(),
    )
}

#[test]
fn a_report_starts_open_and_keeps_the_name_it_was_sent_with() {
    let report = report_user(ReportReason::Spam, None).unwrap();
    assert_eq!(report.status().as_str(), "open");
    assert_eq!(report.target_name(), "offender");
    assert_eq!(report.reason().as_str(), "spam");
}

#[test]
fn nobody_can_report_themselves() {
    let me = UserId::generate();
    let result = Report::create(
        me,
        ReportTarget::User,
        me.uuid(),
        "me".to_string(),
        ReportReason::Spam,
        None,
        ReportContext::default(),
        Utc::now(),
    );
    assert!(matches!(result, Err(ReportError::SelfReport)));
}

#[test]
fn a_community_cannot_be_reported_for_a_personal_reason() {
    let result = Report::create(
        UserId::generate(),
        ReportTarget::Community,
        Uuid::new_v4(),
        "community".to_string(),
        ReportReason::Impersonation,
        None,
        ReportContext::default(),
        Utc::now(),
    );
    assert!(matches!(result, Err(ReportError::ReasonNotAllowed)));
    assert!(!ReportReason::Raid.allowed_for(ReportTarget::User));
    assert!(ReportReason::Raid.allowed_for(ReportTarget::Community));
}

#[test]
fn sticker_packs_accept_content_reasons_only() {
    assert_eq!(ReportTarget::parse("sticker_pack"), Some(ReportTarget::StickerPack));
    assert_eq!(ReportTarget::StickerPack.as_str(), "sticker_pack");
    assert!(ReportReason::Nsfw.allowed_for(ReportTarget::StickerPack));
    assert!(ReportReason::Hate.allowed_for(ReportTarget::StickerPack));
    assert!(!ReportReason::Raid.allowed_for(ReportTarget::StickerPack));
    assert!(!ReportReason::SelfHarm.allowed_for(ReportTarget::StickerPack));
    assert!(!ReportReason::Threats.allowed_for(ReportTarget::StickerPack));
}

#[test]
fn other_needs_an_explanation() {
    assert!(matches!(
        report_user(ReportReason::Other, None),
        Err(ReportError::DetailsRequired)
    ));
    assert!(matches!(
        report_user(ReportReason::Other, Some("   ")),
        Err(ReportError::DetailsRequired)
    ));
    assert!(report_user(ReportReason::Other, Some("he keeps doing it")).is_ok());
}

#[test]
fn blank_details_are_dropped_and_long_ones_rejected() {
    let report = report_user(ReportReason::Spam, Some("  spam link  ")).unwrap();
    assert_eq!(report.details(), Some("spam link"));

    let report = report_user(ReportReason::Spam, Some("   ")).unwrap();
    assert_eq!(report.details(), None);

    let too_long = "я".repeat(MAX_DETAILS_CHARS + 1);
    assert!(matches!(
        report_user(ReportReason::Spam, Some(&too_long)),
        Err(ReportError::DetailsTooLong)
    ));
}

#[test]
fn reasons_survive_a_round_trip_through_their_codes() {
    for reason in [
        ReportReason::Spam,
        ReportReason::Harassment,
        ReportReason::Hate,
        ReportReason::Threats,
        ReportReason::Scam,
        ReportReason::Nsfw,
        ReportReason::Csam,
        ReportReason::SelfHarm,
        ReportReason::Impersonation,
        ReportReason::Illegal,
        ReportReason::Violence,
        ReportReason::Raid,
        ReportReason::Other,
    ] {
        assert_eq!(ReportReason::parse(reason.as_str()), Some(reason));
    }
    assert_eq!(ReportReason::parse("nonsense"), None);
    assert!(ReportReason::Csam.needs_priority_review());
}
