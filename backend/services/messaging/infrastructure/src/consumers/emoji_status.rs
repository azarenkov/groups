use contracts::events::user::UserProfileChangedV1;
use domain::user::id::UserId;
use messaging_domain::emoji_status::{EmojiStatus, EmojiStatusRepository};

pub async fn apply<R: EmojiStatusRepository + ?Sized>(
    statuses: &R,
    subject: &str,
    payload: &[u8],
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if !subject.ends_with(".profile_changed") {
        return Ok(());
    }
    let Ok(evt) = serde_json::from_slice::<UserProfileChangedV1>(payload) else {
        tracing::warn!(%subject, "Malformed profile event; dropping");
        return Ok(());
    };
    let emoji_status = evt.emoji_status_included.then(|| {
        evt.emoji_status_sticker_id.map(|sticker_id| EmojiStatus {
            sticker_id,
            until: evt.emoji_status_until,
        })
    });
    statuses
        .apply_emoji_status(&UserId::new(evt.user_id), emoji_status, evt.occurred_at)
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use async_trait::async_trait;
    use chrono::{DateTime, Utc};
    use messaging_domain::emoji_status::UserEmojiStatus;
    use ports::repository::RepositoryError;
    use uuid::Uuid;

    use super::*;

    #[derive(Default)]
    struct Statuses(Mutex<Vec<Option<Option<EmojiStatus>>>>);

    #[async_trait]
    impl EmojiStatusRepository for Statuses {
        async fn apply_emoji_status(&self, _: &UserId, emoji_status: Option<Option<EmojiStatus>>, _: DateTime<Utc>) -> Result<(), RepositoryError> {
            self.0.lock().unwrap().push(emoji_status);
            Ok(())
        }
        async fn emoji_statuses(&self, _: &[UserId], _: DateTime<Utc>) -> Result<Vec<UserEmojiStatus>, RepositoryError> {
            Ok(Vec::new())
        }
    }

    fn event(extra: serde_json::Value) -> Vec<u8> {
        let mut base = serde_json::json!({
            "user_id": Uuid::nil(),
            "bio": "",
            "show_activity": true,
            "occurred_at": Utc::now(),
        });
        for (k, v) in extra.as_object().unwrap() {
            base[k] = v.clone();
        }
        serde_json::to_vec(&base).unwrap()
    }

    #[tokio::test]
    async fn events_from_older_identities_keep_the_status_and_new_ones_set_or_clear_it() {
        let statuses = Statuses::default();
        let sticker = Uuid::new_v4();
        apply(&statuses, "rift.user.profile_changed", &event(serde_json::json!({}))).await.unwrap();
        apply(
            &statuses,
            "rift.user.profile_changed",
            &event(serde_json::json!({ "emoji_status_included": true, "emoji_status_sticker_id": sticker })),
        )
        .await
        .unwrap();
        apply(&statuses, "rift.user.profile_changed", &event(serde_json::json!({ "emoji_status_included": true })))
            .await
            .unwrap();
        let seen = statuses.0.lock().unwrap().clone();
        assert_eq!(
            seen,
            vec![None, Some(Some(EmojiStatus { sticker_id: sticker, until: None })), Some(None)]
        );
    }

    #[tokio::test]
    async fn other_user_events_are_ignored() {
        let statuses = Statuses::default();
        apply(&statuses, "rift.user.renamed", b"{}").await.unwrap();
        assert!(statuses.0.lock().unwrap().is_empty());
    }
}
