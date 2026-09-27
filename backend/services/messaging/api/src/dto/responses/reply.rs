use messaging::dto::responses::reply::ReplyPreviewDto;
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplyPreviewResponse {
    pub id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author_name: Option<String>,
    pub content: String,
    pub has_attachments: bool,
    pub deleted: bool,
}

impl ReplyPreviewResponse {
    pub fn deleted(id: Uuid) -> Self {
        Self {
            id,
            author_id: None,
            author_name: None,
            content: String::new(),
            has_attachments: false,
            deleted: true,
        }
    }
}

impl From<ReplyPreviewDto> for ReplyPreviewResponse {
    fn from(value: ReplyPreviewDto) -> Self {
        Self {
            id: value.id,
            author_id: Some(value.author_id),
            author_name: value.author_name,
            content: value.content,
            has_attachments: value.has_attachments,
            deleted: false,
        }
    }
}

pub trait RepliesTo {
    fn reply_to_id(&self) -> Option<Uuid>;
    fn set_reply(&mut self, reply: ReplyPreviewResponse);
}

pub fn reply_ids<T: RepliesTo>(items: &[T]) -> Vec<Uuid> {
    let mut ids: Vec<Uuid> = items.iter().filter_map(RepliesTo::reply_to_id).collect();
    ids.sort_unstable();
    ids.dedup();
    ids
}

pub fn fill_replies<T: RepliesTo>(items: &mut [T], previews: Vec<ReplyPreviewDto>) {
    for item in items.iter_mut() {
        let Some(id) = item.reply_to_id() else {
            continue;
        };
        let reply = previews
            .iter()
            .find(|p| p.id == id)
            .cloned()
            .map(ReplyPreviewResponse::from)
            .unwrap_or_else(|| ReplyPreviewResponse::deleted(id));
        item.set_reply(reply);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct Item {
        reply_to_id: Option<Uuid>,
        reply: Option<ReplyPreviewResponse>,
    }

    impl RepliesTo for Item {
        fn reply_to_id(&self) -> Option<Uuid> {
            self.reply_to_id
        }
        fn set_reply(&mut self, reply: ReplyPreviewResponse) {
            self.reply = Some(reply);
        }
    }

    #[test]
    fn fills_the_preview_and_marks_missing_originals_deleted() {
        let kept = Uuid::new_v4();
        let gone = Uuid::new_v4();
        let mut items = vec![
            Item { reply_to_id: Some(kept), ..Default::default() },
            Item { reply_to_id: Some(gone), ..Default::default() },
            Item::default(),
        ];
        let author = Uuid::new_v4();
        fill_replies(
            &mut items,
            vec![ReplyPreviewDto {
                id: kept,
                author_id: author,
                author_name: Some("Ann".into()),
                content: "hi".into(),
                has_attachments: false,
            }],
        );
        let first = items[0].reply.as_ref().unwrap();
        assert!(!first.deleted);
        assert_eq!(first.author_id, Some(author));
        assert_eq!(first.content, "hi");
        assert_eq!(items[1].reply, Some(ReplyPreviewResponse::deleted(gone)));
        assert!(items[2].reply.is_none());
    }

    #[test]
    fn collects_each_replied_message_once() {
        let id = Uuid::new_v4();
        let items = vec![
            Item { reply_to_id: Some(id), ..Default::default() },
            Item { reply_to_id: Some(id), ..Default::default() },
        ];
        assert_eq!(reply_ids(&items), vec![id]);
    }
}
