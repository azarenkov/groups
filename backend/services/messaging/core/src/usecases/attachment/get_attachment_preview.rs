use std::sync::Arc;

use domain::{attachment::preview_key, user::id::UserId};
use futures::stream;
use messaging_domain::{
    attachment_preview::{PREVIEW_SOURCE_MAX_BYTES, PreviewSource, preview_source, rendition_input_name},
    message::repository::MessageRepository,
};
use ports::repository::RepositoryError;
use tokio::sync::Semaphore;
use uuid::Uuid;

use super::derived_objects::{DerivedObjects, KeyLock, SourceRead};
use crate::ports::{
    document_renderer::{DocumentRenderError, DocumentRenderer},
    file_storage::{ByteStream, FileStorage, FileStorageError},
    storage_releases::StorageReleases,
};

pub const PREVIEW_CONCURRENCY: usize = 2;
pub const PREVIEW_CONTENT_TYPE: &str = "application/pdf";

#[derive(Debug, thiserror::Error)]
pub enum GetAttachmentPreviewError {
    #[error("Attachment not found")]
    NotFound,
    #[error("This file type has no preview")]
    Unsupported,
    #[error("The file is too large to preview")]
    TooLarge,
    #[error("The preview could not be created from this file")]
    Unrenderable,
    #[error("Preview service is unavailable")]
    Unavailable,
    #[error("Storage error: {0}")]
    Storage(#[from] FileStorageError),
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
}

impl From<DocumentRenderError> for GetAttachmentPreviewError {
    fn from(error: DocumentRenderError) -> Self {
        match error {
            DocumentRenderError::Rejected(_) => Self::Unrenderable,
            DocumentRenderError::Disabled | DocumentRenderError::Unavailable(_) => Self::Unavailable,
        }
    }
}

struct Inner<M, S, D, R> {
    objects: DerivedObjects<M, S, R>,
    renderer: Arc<D>,
    rendering: Arc<Semaphore>,
}

pub struct GetAttachmentPreview<M, S, D, R> {
    inner: Arc<Inner<M, S, D, R>>,
}

impl<M, S, D, R> GetAttachmentPreview<M, S, D, R>
where
    M: MessageRepository + Send + Sync + 'static,
    S: FileStorage + Send + Sync + 'static,
    D: DocumentRenderer + Send + Sync + 'static,
    R: StorageReleases + Send + Sync + 'static,
{
    pub fn new(message_repository: Arc<M>, storage: Arc<S>, renderer: Arc<D>, releases: Arc<R>) -> Self {
        Self {
            inner: Arc::new(Inner {
                objects: DerivedObjects::new(message_repository, storage, releases),
                renderer,
                rendering: Arc::new(Semaphore::new(PREVIEW_CONCURRENCY)),
            }),
        }
    }

    pub async fn execute(&self, attachment_id: Uuid, user_id: UserId) -> Result<ByteStream, GetAttachmentPreviewError> {
        let inner = &self.inner;
        let link = inner
            .objects
            .linked(attachment_id, &user_id)
            .await?
            .ok_or(GetAttachmentPreviewError::NotFound)?;
        let source = preview_source(&link.content_type, &link.file_name).ok_or(GetAttachmentPreviewError::Unsupported)?;
        if link.size_bytes > PREVIEW_SOURCE_MAX_BYTES {
            return Err(GetAttachmentPreviewError::TooLarge);
        }
        if source == PreviewSource::Original {
            return Ok(inner.objects.storage().get_stream(&link.storage_key).await?);
        }
        if let Some(cached) = inner.objects.open(&preview_key(&link.storage_key)).await? {
            return Ok(cached);
        }
        let input_name = rendition_input_name(&link.content_type, &link.file_name).ok_or(GetAttachmentPreviewError::Unsupported)?;
        let key_lock = inner.objects.lock(&link.storage_key);
        let worker = inner.clone();
        let storage_key = link.storage_key.clone();
        tokio::spawn(async move { worker.render(attachment_id, user_id, storage_key, input_name, key_lock).await })
            .await
            .map_err(|_| GetAttachmentPreviewError::Unavailable)?
    }
}

impl<M, S, D, R> Inner<M, S, D, R>
where
    M: MessageRepository + Send + Sync,
    S: FileStorage + Send + Sync,
    D: DocumentRenderer + Send + Sync,
    R: StorageReleases + Send + Sync,
{
    async fn render(
        &self,
        attachment_id: Uuid,
        user_id: UserId,
        storage_key: String,
        input_name: String,
        key_lock: KeyLock,
    ) -> Result<ByteStream, GetAttachmentPreviewError> {
        let _key_guard = key_lock.hold().await;
        let rendition_key = preview_key(&storage_key);
        if let Some(cached) = self.objects.open(&rendition_key).await? {
            return Ok(cached);
        }
        let _permit = self
            .rendering
            .clone()
            .acquire_owned()
            .await
            .map_err(|_| GetAttachmentPreviewError::Unavailable)?;
        let SourceRead::Bytes(document) = self.objects.read_source(&storage_key, PREVIEW_SOURCE_MAX_BYTES).await? else {
            return Err(GetAttachmentPreviewError::TooLarge);
        };
        let pdf = self.renderer.render_pdf(&input_name, document).await.map_err(|e| {
            tracing::warn!(error = %e, %attachment_id, "Attachment preview rendering failed");
            GetAttachmentPreviewError::from(e)
        })?;
        self.objects
            .persist(attachment_id, &user_id, &storage_key, &rendition_key, PREVIEW_CONTENT_TYPE, pdf.clone())
            .await;
        Ok(Box::pin(stream::once(async move { Ok(pdf) })))
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::HashMap,
        sync::{
            Mutex,
            atomic::{AtomicUsize, Ordering},
        },
    };

    use async_trait::async_trait;
    use chrono::Utc;
    use domain::{
        attachment::AttachmentLink,
        pagination::{PaginatedResult, Pagination},
    };
    use futures::StreamExt;
    use messaging_domain::{
        channel::id::ChannelId,
        message::{Message, id::MessageId, view::{MessageView, ReplyPreviewView}},
    };

    use super::*;

    const DOCX: &str = "application/vnd.openxmlformats-officedocument.wordprocessingml.document";

    struct Messages {
        links: Vec<(UserId, AttachmentLink)>,
    }

    #[async_trait]
    impl MessageRepository for Messages {
        async fn find_reply_author(&self, _: &MessageId) -> Result<Option<UserId>, RepositoryError> {
            unimplemented!()
        }
        async fn list_users_hiding(&self, _: Uuid) -> Result<Vec<UserId>, RepositoryError> {
            unimplemented!()
        }
        async fn list_attachment_links_for_messages(&self, _: &[Uuid]) -> Result<Vec<(Uuid, AttachmentLink)>, RepositoryError> {
            unimplemented!()
        }
        async fn find_attachment_for_user(&self, attachment_id: Uuid, user_id: &UserId) -> Result<Option<AttachmentLink>, RepositoryError> {
            Ok(self
                .links
                .iter()
                .find(|(owner, link)| owner == user_id && link.id == attachment_id)
                .map(|(_, link)| link.clone()))
        }
        async fn linked_attachment_subset(&self, _: &[Uuid]) -> Result<Vec<Uuid>, RepositoryError> {
            unimplemented!()
        }
        async fn find_message_by_id(&self, _: &MessageId) -> Result<Message, RepositoryError> {
            unimplemented!()
        }
        async fn find_message_view(&self, _: &MessageId, _: &UserId) -> Result<MessageView, RepositoryError> {
            unimplemented!()
        }
        async fn list_reply_previews(&self, _: &[Uuid], _: &UserId) -> Result<Vec<ReplyPreviewView>, RepositoryError> {
            unimplemented!()
        }
        async fn list_messages_in_channel(
            &self,
            _: &ChannelId,
            _: &UserId,
            _: Option<&MessageId>,
            _: Option<&MessageId>,
            _: i64,
        ) -> Result<Vec<MessageView>, RepositoryError> {
            unimplemented!()
        }
        async fn search_messages_in_channel(
            &self,
            _: &ChannelId,
            _: &UserId,
            _: &str,
            _: Pagination,
        ) -> Result<PaginatedResult<MessageView>, RepositoryError> {
            unimplemented!()
        }
    }

    #[derive(Default)]
    struct Bucket {
        objects: Mutex<HashMap<String, Vec<u8>>>,
    }

    #[async_trait]
    impl FileStorage for Bucket {
        async fn put(&self, key: &str, _: &str, bytes: Vec<u8>) -> Result<(), FileStorageError> {
            self.objects.lock().unwrap().insert(key.to_string(), bytes);
            Ok(())
        }
        async fn put_stream(&self, _: &str, _: &str, _: ByteStream) -> Result<u64, FileStorageError> {
            unimplemented!()
        }
        async fn get_stream(&self, key: &str) -> Result<ByteStream, FileStorageError> {
            let bytes = self.objects.lock().unwrap().get(key).cloned().ok_or(FileStorageError::NotFound)?;
            Ok(Box::pin(stream::once(async move { Ok(bytes) })))
        }
        async fn delete(&self, key: &str) -> Result<(), FileStorageError> {
            self.objects.lock().unwrap().remove(key);
            Ok(())
        }
    }

    struct Renderer {
        calls: AtomicUsize,
        outcome: fn() -> Result<Vec<u8>, DocumentRenderError>,
        inputs: Mutex<Vec<String>>,
    }

    impl Renderer {
        fn new(outcome: fn() -> Result<Vec<u8>, DocumentRenderError>) -> Self {
            Self { calls: AtomicUsize::new(0), outcome, inputs: Mutex::new(Vec::new()) }
        }
    }

    #[async_trait]
    impl DocumentRenderer for Renderer {
        async fn render_pdf(&self, input_name: &str, _: Vec<u8>) -> Result<Vec<u8>, DocumentRenderError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            self.inputs.lock().unwrap().push(input_name.to_string());
            (self.outcome)()
        }
    }

    struct Releases;

    #[async_trait]
    impl StorageReleases for Releases {
        async fn release(&self, _: &[String]) -> Result<(), RepositoryError> {
            Ok(())
        }
    }

    fn link(content_type: &str, file_name: &str, size_bytes: i64) -> AttachmentLink {
        AttachmentLink {
            id: Uuid::new_v4(),
            file_name: file_name.to_string(),
            content_type: content_type.to_string(),
            size_bytes,
            storage_key: format!("attachments/{}", Uuid::new_v4()),
            created_at: Utc::now(),
            voice: None,
            video_note: None,
        }
    }

    fn setup(
        viewer: UserId,
        attachment: &AttachmentLink,
        renderer: Arc<Renderer>,
    ) -> (GetAttachmentPreview<Messages, Bucket, Renderer, Releases>, Arc<Bucket>) {
        let bucket = Arc::new(Bucket::default());
        bucket.objects.lock().unwrap().insert(attachment.storage_key.clone(), b"original".to_vec());
        let messages = Arc::new(Messages { links: vec![(viewer, attachment.clone())] });
        (GetAttachmentPreview::new(messages, bucket.clone(), renderer, Arc::new(Releases)), bucket)
    }

    async fn body(stream: ByteStream) -> Vec<u8> {
        stream.map(|chunk| chunk.unwrap()).concat().await
    }

    #[tokio::test]
    async fn a_pdf_is_served_as_is_without_rendering() {
        let viewer = UserId::generate();
        let attachment = link("application/pdf", "notes.pdf", 8);
        let renderer = Arc::new(Renderer::new(|| Ok(b"never".to_vec())));
        let (usecase, _) = setup(viewer, &attachment, renderer.clone());
        let preview = usecase.execute(attachment.id, viewer).await.unwrap();
        assert_eq!(body(preview).await, b"original");
        assert_eq!(renderer.calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn an_office_document_is_rendered_once_and_then_served_from_storage() {
        let viewer = UserId::generate();
        let attachment = link(DOCX, "Essay.docx", 8);
        let renderer = Arc::new(Renderer::new(|| Ok(b"%PDF-rendition".to_vec())));
        let (usecase, bucket) = setup(viewer, &attachment, renderer.clone());
        assert_eq!(body(usecase.execute(attachment.id, viewer).await.unwrap()).await, b"%PDF-rendition");
        assert_eq!(body(usecase.execute(attachment.id, viewer).await.unwrap()).await, b"%PDF-rendition");
        assert_eq!(renderer.calls.load(Ordering::SeqCst), 1);
        assert_eq!(*renderer.inputs.lock().unwrap(), vec!["document.docx".to_string()]);
        assert!(bucket.objects.lock().unwrap().contains_key(&preview_key(&attachment.storage_key)));
    }

    #[tokio::test]
    async fn a_rejected_document_reports_that_no_preview_can_be_made_and_stores_nothing() {
        let viewer = UserId::generate();
        let attachment = link(DOCX, "locked.docx", 8);
        let renderer = Arc::new(Renderer::new(|| Err(DocumentRenderError::Rejected("password protected".into()))));
        let (usecase, bucket) = setup(viewer, &attachment, renderer);
        assert!(matches!(usecase.execute(attachment.id, viewer).await, Err(GetAttachmentPreviewError::Unrenderable)));
        assert!(!bucket.objects.lock().unwrap().contains_key(&preview_key(&attachment.storage_key)));
    }

    #[tokio::test]
    async fn a_missing_renderer_reports_the_service_as_unavailable() {
        let viewer = UserId::generate();
        let attachment = link(DOCX, "a.docx", 8);
        let (usecase, _) = setup(viewer, &attachment, Arc::new(Renderer::new(|| Err(DocumentRenderError::Disabled))));
        assert!(matches!(usecase.execute(attachment.id, viewer).await, Err(GetAttachmentPreviewError::Unavailable)));
    }

    #[tokio::test]
    async fn previews_follow_access_type_and_size_rules() {
        let viewer = UserId::generate();
        let renderer = Arc::new(Renderer::new(|| Ok(Vec::new())));
        let zip = link("application/zip", "a.zip", 8);
        let (usecase, _) = setup(viewer, &zip, renderer.clone());
        assert!(matches!(usecase.execute(zip.id, viewer).await, Err(GetAttachmentPreviewError::Unsupported)));
        assert!(matches!(usecase.execute(zip.id, UserId::generate()).await, Err(GetAttachmentPreviewError::NotFound)));
        let huge = link("application/pdf", "big.pdf", PREVIEW_SOURCE_MAX_BYTES + 1);
        let (usecase, _) = setup(viewer, &huge, renderer.clone());
        assert!(matches!(usecase.execute(huge.id, viewer).await, Err(GetAttachmentPreviewError::TooLarge)));
        assert_eq!(renderer.calls.load(Ordering::SeqCst), 0);
    }
}
