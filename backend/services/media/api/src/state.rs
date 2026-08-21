use media::usecases::{
    attachment::{delete_attachment::DeleteAttachment, upload_attachment::UploadAttachment},
    server::{get_logo::GetServerLogo, update_logo::UpdateServerLogo},
    user::{
        delete_avatar::DeleteUserAvatar, get_avatar::GetUserAvatar,
        update_avatar::UpdateUserAvatar,
    },
};
use media::{
    database::postgres::repositories::{
        attachment::PostgresAttachmentRepository, media::PostgresMediaRepository,
    },
    media_client::owner_lookup::MessagingOwnerClient,
    moderation::nsfw_detector::NsfwDetector,
    storage::minio_storage::MinioStorage,
};

pub struct AppState {
    pub upload_attachment_usecase: UploadAttachment<PostgresAttachmentRepository, MinioStorage>,
    pub upload_semaphore: std::sync::Arc<tokio::sync::Semaphore>,
    pub image_read_limiter: shared::rate_limiting::KeyedRateLimiter,
    pub delete_attachment_usecase: DeleteAttachment<PostgresAttachmentRepository, MinioStorage>,
    pub attachment_repository: std::sync::Arc<PostgresAttachmentRepository>,
    pub update_user_avatar_usecase: UpdateUserAvatar<
        PostgresMediaRepository,
        PostgresAttachmentRepository,
        MinioStorage,
        NsfwDetector,
    >,
    pub delete_user_avatar_usecase: DeleteUserAvatar<
        PostgresMediaRepository,
        PostgresAttachmentRepository,
        MinioStorage,
    >,
    pub get_user_avatar_usecase: GetUserAvatar<PostgresMediaRepository, MinioStorage>,
    pub update_server_logo_usecase: UpdateServerLogo<
        PostgresMediaRepository,
        PostgresAttachmentRepository,
        MinioStorage,
        NsfwDetector,
        MessagingOwnerClient,
    >,
    pub get_server_logo_usecase: GetServerLogo<PostgresMediaRepository, MinioStorage>,
}
