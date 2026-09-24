use actix_web::{HttpResponse, ResponseError};
use messaging_domain::{
    channel::ChannelError, dm_message::DmMessageError, friendship::FriendshipError,
    message::MessageError,
};
use api_common::api_error::{ApiError, StatusCode, ToHttpStatus};

pub trait HttpMapping {
    fn to_http_status(&self) -> StatusCode;
    fn to_user_message(&self) -> String;
    fn to_error_code(&self) -> &'static str;
}

#[derive(Debug)]
pub struct HttpError(pub ApiError);

impl std::fmt::Display for HttpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl ResponseError for HttpError {
    fn status_code(&self) -> StatusCode {
        self.0.status_code()
    }

    fn error_response(&self) -> HttpResponse {
        self.0.error_response()
    }
}

#[allow(dead_code)]
impl HttpError {
    pub fn new(message: String, status: StatusCode, code: &'static str) -> Self {
        Self(ApiError::new(message, status, code))
    }

    pub fn bad_request(message: String) -> Self {
        Self(ApiError::bad_request(message))
    }

    pub fn payload_too_large() -> Self {
        Self(ApiError::payload_too_large())
    }

    pub fn from_error<E: ToHttpStatus + std::fmt::Debug>(error: E) -> Self {
        Self(ApiError::from_error(error))
    }
}

impl From<ApiError> for HttpError {
    fn from(error: ApiError) -> Self {
        Self(error)
    }
}

impl From<validator::ValidationErrors> for HttpError {
    fn from(error: validator::ValidationErrors) -> Self {
        Self(ApiError::from(error))
    }
}

impl From<ports::repository::RepositoryError> for HttpError {
    fn from(error: ports::repository::RepositoryError) -> Self {
        Self(ApiError::from(error))
    }
}

impl From<api_common::pagination::PaginationError> for HttpError {
    fn from(error: api_common::pagination::PaginationError) -> Self {
        Self(ApiError::from(error))
    }
}

impl<E: HttpMapping + std::fmt::Debug> From<E> for HttpError {
    fn from(error: E) -> Self {
        tracing::error!("API Error: {:?}", error);
        Self(ApiError::new(
            error.to_user_message(),
            error.to_http_status(),
            error.to_error_code(),
        ))
    }
}

macro_rules! impl_http_mapping_delegate {
    ($err:ident, $variant:ident) => {
        impl HttpMapping for $err {
            fn to_http_status(&self) -> StatusCode {
                let $err::$variant(e) = self;
                e.to_http_status()
            }

            fn to_user_message(&self) -> String {
                let $err::$variant(e) = self;
                e.to_user_message()
            }

            fn to_error_code(&self) -> &'static str {
                let $err::$variant(e) = self;
                e.to_error_code()
            }
        }
    };
}

use messaging::usecases::attachment::get_attachment::GetAttachmentError;
use messaging::usecases::attachment::list_message_attachments::ListMessageAttachmentsError;
use messaging::usecases::call::log_call::LogCallError;
use messaging::usecases::channel::abort_voice_connection::AbortVoiceConnectionError;
use messaging::usecases::channel::create_channel::CreateChannelError;
use messaging::usecases::channel::delete_channel::DeleteChannelError;
use messaging::usecases::channel::delete_channel_message::DeleteChannelMessageError;
use messaging::usecases::channel::edit_channel_message::EditChannelMessageError;
use messaging::usecases::channel::join_text_channel::JoinTextChannelError;
use messaging::usecases::channel::join_voice_channel::JoinVoiceChannelError;
use messaging::usecases::channel::join_voice_connection::JoinVoiceConnectionError;
use messaging::usecases::channel::leave_voice_connection::LeaveVoiceConnectionError;
use messaging::usecases::channel::list_channel_messages::ListChannelMessagesError;
use messaging::usecases::channel::list_channel_participants::ListChannelParticipantsError;
use messaging::usecases::channel::mute_channel::MuteChannelError;
use messaging::usecases::channel::deafen_channel::DeafenChannelError;
use messaging::usecases::channel::undeafen_channel::UndeafenChannelError;
use messaging::usecases::channel::search_channel_messages::SearchChannelMessagesError;
use messaging::usecases::channel::send_channel_message::SendChannelMessageError;
use messaging::usecases::channel::track_media_connection::TrackMediaConnectionError;
use messaging::usecases::channel::unmute_channel::UnmuteChannelError;
use messaging::usecases::channel::update_stream_viewer::UpdateStreamViewerError;
use messaging::usecases::dm::delete_dm_message::DeleteDmMessageError;
use messaging::usecases::dm::edit_dm_message::EditDmMessageError;
use messaging::usecases::dm::list_dm_messages::ListDmMessagesError;
use messaging::usecases::dm::list_dm_previews::ListDmPreviewsError;
use messaging::usecases::dm::search_dm_messages::SearchDmMessagesError;
use messaging::usecases::dm::send_dm_message::SendDmMessageError;
use messaging::usecases::friendship::accept_friend_request::AcceptFriendRequestError;
use messaging::usecases::friendship::list_friends::ListFriendsError;
use messaging::usecases::friendship::remove_friendship::RemoveFriendshipError;
use messaging::usecases::friendship::send_friend_request::SendFriendRequestError;
use messaging::usecases::notification::list_notification_preferences::ListNotificationPreferencesError;
use messaging::usecases::notification::set_notification_preference::SetNotificationPreferenceError;
use messaging::usecases::read_state::get_channel_read_receipts::GetChannelReadReceiptsError;
use messaging::usecases::role::RoleError;
use messaging::usecases::read_state::get_dm_read_receipt::GetDmReadReceiptError;
use messaging::usecases::read_state::get_unread_counts::GetUnreadCountsError;
use messaging::usecases::read_state::mark_channel_read::MarkChannelReadError;
use messaging::usecases::read_state::mark_dm_read::MarkDmReadError;
use messaging::usecases::community::accept_community_invite::AcceptCommunityInviteError;
use messaging::usecases::community::create_community::CreateCommunityError;
use messaging::usecases::community::decline_community_invite::DeclineCommunityInviteError;
use messaging::usecases::community::delete_community::DeleteCommunityError;
use messaging::usecases::community::get_voice_presence::GetVoicePresenceError;
use messaging::usecases::community::invite_links::JoinByInviteLinkError;
use messaging::usecases::block::block_user::BlockUserError;
use messaging::usecases::block::unblock_user::UnblockUserError;
use messaging::usecases::community::invite_links::ManageInviteLinksError;
use messaging::usecases::community::invite_links::PreviewInviteLinkError;
use messaging::usecases::community::invite_links::RevokeInviteLinkError;
use messaging_domain::community::invite_link::InviteLinkStatus;
use messaging::usecases::community::get_home_overview::GetHomeOverviewError;
use messaging::usecases::privacy::get_privacy::GetPrivacyError;
use messaging::usecases::report::create_report::CreateReportError;
use messaging_domain::report::ReportError;
use messaging::usecases::privacy::update_privacy::UpdatePrivacyError;
use messaging::usecases::community::get_user_profile::GetUserProfileError;
use messaging::usecases::community::kick_member::KickMemberError;
use messaging::usecases::community::leave_community::LeaveCommunityError;
use messaging::usecases::community::list_sent_community_invites::ListSentCommunityInvitesError;
use messaging::usecases::community::list_community_channels::ListCommunityChannelsError;
use messaging::usecases::community::list_community_members::ListCommunityMembersError;
use messaging::usecases::community::list_communities_by_user::ListCommunitiesByUserError;
use messaging::usecases::community::send_community_invite::SendCommunityInviteError;
use messaging::usecases::community::update_community::UpdateCommunityError;

impl HttpMapping for CreateChannelError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            CreateChannelError::Repository(e) => e.to_http_status(),
            CreateChannelError::Hash(e) => e.to_http_status(),
            CreateChannelError::NotAllowed => StatusCode::FORBIDDEN,
            CreateChannelError::PasswordRequired => StatusCode::BAD_REQUEST,
            CreateChannelError::InvalidData => StatusCode::BAD_REQUEST,
            CreateChannelError::Domain(e) => e.to_http_status(),
        }
    }

    fn to_user_message(&self) -> String {
        match self {
            CreateChannelError::Repository(e) => e.to_user_message(),
            CreateChannelError::Hash(e) => e.to_user_message(),
            CreateChannelError::NotAllowed => {
                "You don't have permission to manage channels".to_string()
            }
            CreateChannelError::PasswordRequired => {
                "Password is required for private channels".to_string()
            }
            CreateChannelError::InvalidData => "Invalid channel data provided".to_string(),
            CreateChannelError::Domain(e) => e.to_user_message(),
        }
    }

    fn to_error_code(&self) -> &'static str {
        match self {
            CreateChannelError::Repository(e) => e.to_error_code(),
            CreateChannelError::Hash(e) => e.to_error_code(),
            CreateChannelError::NotAllowed => "CHANNEL_NOT_ALLOWED",
            CreateChannelError::PasswordRequired => "CHANNEL_PASSWORD_REQUIRED",
            CreateChannelError::InvalidData => "CHANNEL_INVALID_DATA",
            CreateChannelError::Domain(e) => e.to_error_code(),
        }
    }
}

impl HttpMapping for DeleteChannelError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            DeleteChannelError::ChannelNotFound => StatusCode::NOT_FOUND,
            DeleteChannelError::NotOwner => StatusCode::FORBIDDEN,
            DeleteChannelError::Repository(e) => e.to_http_status(),
            DeleteChannelError::Domain(e) => e.to_http_status(),
            DeleteChannelError::VoiceService(_) => StatusCode::BAD_GATEWAY,
        }
    }

    fn to_user_message(&self) -> String {
        match self {
            DeleteChannelError::ChannelNotFound => "Channel not found".to_string(),
            DeleteChannelError::NotOwner => "Only the channel owner can delete it.".to_string(),
            DeleteChannelError::Repository(e) => e.to_user_message(),
            DeleteChannelError::Domain(e) => e.to_user_message(),
            DeleteChannelError::VoiceService(_) => {
                "Voice service is temporarily unavailable.".to_string()
            }
        }
    }

    fn to_error_code(&self) -> &'static str {
        match self {
            DeleteChannelError::ChannelNotFound => "CHANNEL_NOT_FOUND",
            DeleteChannelError::NotOwner => "CHANNEL_NOT_OWNER",
            DeleteChannelError::Repository(e) => e.to_error_code(),
            DeleteChannelError::Domain(e) => e.to_error_code(),
            DeleteChannelError::VoiceService(e) => e.to_error_code(),
        }
    }
}

impl HttpMapping for MuteChannelError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            MuteChannelError::ChannelNotFound | MuteChannelError::TargetUserNotFound => {
                StatusCode::NOT_FOUND
            }
            MuteChannelError::VoiceNotSupported => StatusCode::BAD_REQUEST,
            MuteChannelError::Repository(e) => e.to_http_status(),
            MuteChannelError::Domain(e) => e.to_http_status(),
        }
    }

    fn to_user_message(&self) -> String {
        match self {
            MuteChannelError::ChannelNotFound => "Channel not found".to_string(),
            MuteChannelError::TargetUserNotFound => "Target user not found".to_string(),
            MuteChannelError::VoiceNotSupported => {
                "Voice is not supported for this channel".to_string()
            }
            MuteChannelError::Repository(e) => e.to_user_message(),
            MuteChannelError::Domain(e) => e.to_user_message(),
        }
    }

    fn to_error_code(&self) -> &'static str {
        match self {
            MuteChannelError::ChannelNotFound => "CHANNEL_NOT_FOUND",
            MuteChannelError::TargetUserNotFound => "TARGET_USER_NOT_FOUND",
            MuteChannelError::VoiceNotSupported => "VOICE_NOT_SUPPORTED",
            MuteChannelError::Repository(e) => e.to_error_code(),
            MuteChannelError::Domain(e) => e.to_error_code(),
        }
    }
}

impl HttpMapping for DeafenChannelError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            DeafenChannelError::ChannelNotFound | DeafenChannelError::TargetUserNotFound => {
                StatusCode::NOT_FOUND
            }
            DeafenChannelError::VoiceNotSupported => StatusCode::BAD_REQUEST,
            DeafenChannelError::Repository(e) => e.to_http_status(),
            DeafenChannelError::Domain(e) => e.to_http_status(),
        }
    }

    fn to_user_message(&self) -> String {
        match self {
            DeafenChannelError::ChannelNotFound => "Channel not found".to_string(),
            DeafenChannelError::TargetUserNotFound => "Target user not found".to_string(),
            DeafenChannelError::VoiceNotSupported => {
                "Voice is not supported for this channel".to_string()
            }
            DeafenChannelError::Repository(e) => e.to_user_message(),
            DeafenChannelError::Domain(e) => e.to_user_message(),
        }
    }

    fn to_error_code(&self) -> &'static str {
        match self {
            DeafenChannelError::ChannelNotFound => "CHANNEL_NOT_FOUND",
            DeafenChannelError::TargetUserNotFound => "TARGET_USER_NOT_FOUND",
            DeafenChannelError::VoiceNotSupported => "VOICE_NOT_SUPPORTED",
            DeafenChannelError::Repository(e) => e.to_error_code(),
            DeafenChannelError::Domain(e) => e.to_error_code(),
        }
    }
}

impl HttpMapping for UndeafenChannelError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            UndeafenChannelError::ChannelNotFound | UndeafenChannelError::TargetUserNotFound => {
                StatusCode::NOT_FOUND
            }
            UndeafenChannelError::VoiceNotSupported => StatusCode::BAD_REQUEST,
            UndeafenChannelError::Repository(e) => e.to_http_status(),
            UndeafenChannelError::Domain(e) => e.to_http_status(),
        }
    }

    fn to_user_message(&self) -> String {
        match self {
            UndeafenChannelError::ChannelNotFound => "Channel not found".to_string(),
            UndeafenChannelError::TargetUserNotFound => "Target user not found".to_string(),
            UndeafenChannelError::VoiceNotSupported => {
                "Voice is not supported for this channel".to_string()
            }
            UndeafenChannelError::Repository(e) => e.to_user_message(),
            UndeafenChannelError::Domain(e) => e.to_user_message(),
        }
    }

    fn to_error_code(&self) -> &'static str {
        match self {
            UndeafenChannelError::ChannelNotFound => "CHANNEL_NOT_FOUND",
            UndeafenChannelError::TargetUserNotFound => "TARGET_USER_NOT_FOUND",
            UndeafenChannelError::VoiceNotSupported => "VOICE_NOT_SUPPORTED",
            UndeafenChannelError::Repository(e) => e.to_error_code(),
            UndeafenChannelError::Domain(e) => e.to_error_code(),
        }
    }
}

impl HttpMapping for UnmuteChannelError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            UnmuteChannelError::ChannelNotFound | UnmuteChannelError::TargetUserNotFound => {
                StatusCode::NOT_FOUND
            }
            UnmuteChannelError::VoiceNotSupported => StatusCode::BAD_REQUEST,
            UnmuteChannelError::Repository(e) => e.to_http_status(),
            UnmuteChannelError::Domain(e) => e.to_http_status(),
        }
    }

    fn to_user_message(&self) -> String {
        match self {
            UnmuteChannelError::ChannelNotFound => "Channel not found".to_string(),
            UnmuteChannelError::TargetUserNotFound => "Target user not found".to_string(),
            UnmuteChannelError::VoiceNotSupported => {
                "Voice is not supported for this channel".to_string()
            }
            UnmuteChannelError::Repository(e) => e.to_user_message(),
            UnmuteChannelError::Domain(e) => e.to_user_message(),
        }
    }

    fn to_error_code(&self) -> &'static str {
        match self {
            UnmuteChannelError::ChannelNotFound => "CHANNEL_NOT_FOUND",
            UnmuteChannelError::TargetUserNotFound => "TARGET_USER_NOT_FOUND",
            UnmuteChannelError::VoiceNotSupported => "VOICE_NOT_SUPPORTED",
            UnmuteChannelError::Repository(e) => e.to_error_code(),
            UnmuteChannelError::Domain(e) => e.to_error_code(),
        }
    }
}

impl HttpMapping for AbortVoiceConnectionError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            AbortVoiceConnectionError::Repository(e) => e.to_http_status(),
            AbortVoiceConnectionError::Domain(e) => e.to_http_status(),
        }
    }

    fn to_user_message(&self) -> String {
        match self {
            AbortVoiceConnectionError::Repository(e) => e.to_user_message(),
            AbortVoiceConnectionError::Domain(e) => e.to_user_message(),
        }
    }

    fn to_error_code(&self) -> &'static str {
        match self {
            AbortVoiceConnectionError::Repository(e) => e.to_error_code(),
            AbortVoiceConnectionError::Domain(e) => e.to_error_code(),
        }
    }
}

impl HttpMapping for LeaveVoiceConnectionError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            LeaveVoiceConnectionError::Repository(e) => e.to_http_status(),
            LeaveVoiceConnectionError::Domain(e) => e.to_http_status(),
        }
    }

    fn to_user_message(&self) -> String {
        match self {
            LeaveVoiceConnectionError::Repository(e) => e.to_user_message(),
            LeaveVoiceConnectionError::Domain(e) => e.to_user_message(),
        }
    }

    fn to_error_code(&self) -> &'static str {
        match self {
            LeaveVoiceConnectionError::Repository(e) => e.to_error_code(),
            LeaveVoiceConnectionError::Domain(e) => e.to_error_code(),
        }
    }
}

impl HttpMapping for JoinVoiceConnectionError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            JoinVoiceConnectionError::Repository(e) => e.to_http_status(),
            JoinVoiceConnectionError::Domain(e) => e.to_http_status(),
        }
    }

    fn to_user_message(&self) -> String {
        match self {
            JoinVoiceConnectionError::Repository(e) => e.to_user_message(),
            JoinVoiceConnectionError::Domain(e) => e.to_user_message(),
        }
    }

    fn to_error_code(&self) -> &'static str {
        match self {
            JoinVoiceConnectionError::Repository(e) => e.to_error_code(),
            JoinVoiceConnectionError::Domain(e) => e.to_error_code(),
        }
    }
}

impl HttpMapping for TrackMediaConnectionError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            TrackMediaConnectionError::Repository(e) => e.to_http_status(),
            TrackMediaConnectionError::Domain(e) => e.to_http_status(),
        }
    }

    fn to_user_message(&self) -> String {
        match self {
            TrackMediaConnectionError::Repository(e) => e.to_user_message(),
            TrackMediaConnectionError::Domain(e) => e.to_user_message(),
        }
    }

    fn to_error_code(&self) -> &'static str {
        match self {
            TrackMediaConnectionError::Repository(e) => e.to_error_code(),
            TrackMediaConnectionError::Domain(e) => e.to_error_code(),
        }
    }
}

impl HttpMapping for CreateCommunityError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            CreateCommunityError::Repository(e) => e.to_http_status(),
            CreateCommunityError::AccountInactive => StatusCode::FORBIDDEN,
            CreateCommunityError::NameNotAllowed | CreateCommunityError::DescriptionNotAllowed => {
                StatusCode::UNPROCESSABLE_ENTITY
            }
            CreateCommunityError::ModerationUnavailable(_) => StatusCode::SERVICE_UNAVAILABLE,
            CreateCommunityError::Channel(e) => e.to_http_status(),
        }
    }

    fn to_user_message(&self) -> String {
        match self {
            CreateCommunityError::Repository(e) => e.to_user_message(),
            CreateCommunityError::AccountInactive => "Account is inactive".to_string(),
            CreateCommunityError::NameNotAllowed => "Community name is not allowed".to_string(),
            CreateCommunityError::DescriptionNotAllowed => {
                "Community description is not allowed".to_string()
            }
            CreateCommunityError::ModerationUnavailable(_) => {
                "Content moderation is temporarily unavailable".to_string()
            }
            CreateCommunityError::Channel(e) => e.to_user_message(),
        }
    }

    fn to_error_code(&self) -> &'static str {
        match self {
            CreateCommunityError::Repository(e) => e.to_error_code(),
            CreateCommunityError::AccountInactive => "ACCOUNT_INACTIVE",
            CreateCommunityError::NameNotAllowed => "COMMUNITY_NAME_NOT_ALLOWED",
            CreateCommunityError::DescriptionNotAllowed => "COMMUNITY_DESCRIPTION_NOT_ALLOWED",
            CreateCommunityError::ModerationUnavailable(_) => "MODERATION_UNAVAILABLE",
            CreateCommunityError::Channel(e) => e.to_error_code(),
        }
    }
}

impl HttpMapping for DeleteCommunityError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            DeleteCommunityError::CommunityNotFound => StatusCode::NOT_FOUND,
            DeleteCommunityError::NotOwner => StatusCode::FORBIDDEN,
            DeleteCommunityError::Repository(e) => e.to_http_status(),
            DeleteCommunityError::VoiceService(_) => StatusCode::BAD_GATEWAY,
        }
    }

    fn to_user_message(&self) -> String {
        match self {
            DeleteCommunityError::CommunityNotFound => "Community not found".to_string(),
            DeleteCommunityError::NotOwner => "Only the community owner can delete it.".to_string(),
            DeleteCommunityError::Repository(e) => e.to_user_message(),
            DeleteCommunityError::VoiceService(_) => {
                "Voice service is temporarily unavailable.".to_string()
            }
        }
    }

    fn to_error_code(&self) -> &'static str {
        match self {
            DeleteCommunityError::CommunityNotFound => "COMMUNITY_NOT_FOUND",
            DeleteCommunityError::NotOwner => "COMMUNITY_NOT_OWNER",
            DeleteCommunityError::Repository(e) => e.to_error_code(),
            DeleteCommunityError::VoiceService(e) => e.to_error_code(),
        }
    }
}

impl HttpMapping for UpdateCommunityError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            UpdateCommunityError::CommunityNotFound => StatusCode::NOT_FOUND,
            UpdateCommunityError::NotOwner => StatusCode::FORBIDDEN,
            UpdateCommunityError::NameTaken => StatusCode::CONFLICT,
            UpdateCommunityError::NameNotAllowed | UpdateCommunityError::DescriptionNotAllowed => {
                StatusCode::UNPROCESSABLE_ENTITY
            }
            UpdateCommunityError::ModerationUnavailable(_) => StatusCode::SERVICE_UNAVAILABLE,
            UpdateCommunityError::Repository(e) => e.to_http_status(),
        }
    }

    fn to_user_message(&self) -> String {
        match self {
            UpdateCommunityError::CommunityNotFound => "Community not found".to_string(),
            UpdateCommunityError::NotOwner => "Only the community owner can edit it.".to_string(),
            UpdateCommunityError::NameTaken => "A community with this name already exists.".to_string(),
            UpdateCommunityError::NameNotAllowed => "Community name is not allowed".to_string(),
            UpdateCommunityError::DescriptionNotAllowed => {
                "Community description is not allowed".to_string()
            }
            UpdateCommunityError::ModerationUnavailable(_) => {
                "Content moderation is temporarily unavailable".to_string()
            }
            UpdateCommunityError::Repository(e) => e.to_user_message(),
        }
    }

    fn to_error_code(&self) -> &'static str {
        match self {
            UpdateCommunityError::CommunityNotFound => "COMMUNITY_NOT_FOUND",
            UpdateCommunityError::NotOwner => "COMMUNITY_NOT_OWNER",
            UpdateCommunityError::NameTaken => "COMMUNITY_NAME_TAKEN",
            UpdateCommunityError::NameNotAllowed => "COMMUNITY_NAME_NOT_ALLOWED",
            UpdateCommunityError::DescriptionNotAllowed => "COMMUNITY_DESCRIPTION_NOT_ALLOWED",
            UpdateCommunityError::ModerationUnavailable(_) => "MODERATION_UNAVAILABLE",
            UpdateCommunityError::Repository(e) => e.to_error_code(),
        }
    }
}

impl HttpMapping for ManageInviteLinksError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            ManageInviteLinksError::NotMember => StatusCode::FORBIDDEN,
            ManageInviteLinksError::NotAllowed => StatusCode::FORBIDDEN,
            ManageInviteLinksError::Repository(e) => e.to_http_status(),
        }
    }
    fn to_user_message(&self) -> String {
        match self {
            ManageInviteLinksError::NotAllowed => {
                "You don't have permission to manage invites".to_string()
            }
            ManageInviteLinksError::NotMember => {
                "You are not a member of this community.".to_string()
            }
            ManageInviteLinksError::Repository(e) => e.to_user_message(),
        }
    }
    fn to_error_code(&self) -> &'static str {
        match self {
            ManageInviteLinksError::NotMember => "COMMUNITY_NOT_MEMBER",
            ManageInviteLinksError::NotAllowed => "INVITE_NOT_ALLOWED",
            ManageInviteLinksError::Repository(e) => e.to_error_code(),
        }
    }
}

impl HttpMapping for RevokeInviteLinkError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            RevokeInviteLinkError::NotFound => StatusCode::NOT_FOUND,
            RevokeInviteLinkError::Repository(e) => e.to_http_status(),
        }
    }
    fn to_user_message(&self) -> String {
        match self {
            RevokeInviteLinkError::NotFound => "Invite link not found.".to_string(),
            RevokeInviteLinkError::Repository(e) => e.to_user_message(),
        }
    }
    fn to_error_code(&self) -> &'static str {
        match self {
            RevokeInviteLinkError::NotFound => "INVITE_NOT_FOUND",
            RevokeInviteLinkError::Repository(e) => e.to_error_code(),
        }
    }
}

impl HttpMapping for PreviewInviteLinkError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            PreviewInviteLinkError::NotFound => StatusCode::NOT_FOUND,
            PreviewInviteLinkError::Repository(e) => e.to_http_status(),
        }
    }
    fn to_user_message(&self) -> String {
        match self {
            PreviewInviteLinkError::NotFound => "This invite link is invalid.".to_string(),
            PreviewInviteLinkError::Repository(e) => e.to_user_message(),
        }
    }
    fn to_error_code(&self) -> &'static str {
        match self {
            PreviewInviteLinkError::NotFound => "INVITE_NOT_FOUND",
            PreviewInviteLinkError::Repository(e) => e.to_error_code(),
        }
    }
}

impl HttpMapping for JoinByInviteLinkError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            JoinByInviteLinkError::NotFound | JoinByInviteLinkError::CommunityNotFound => {
                StatusCode::NOT_FOUND
            }
            JoinByInviteLinkError::Unavailable(_) => StatusCode::GONE,
            JoinByInviteLinkError::Repository(e) => e.to_http_status(),
        }
    }
    fn to_user_message(&self) -> String {
        match self {
            JoinByInviteLinkError::NotFound => "This invite link is invalid.".to_string(),
            JoinByInviteLinkError::Unavailable(InviteLinkStatus::Expired) => {
                "This invite link has expired.".to_string()
            }
            JoinByInviteLinkError::Unavailable(InviteLinkStatus::Exhausted) => {
                "This invite link has reached its usage limit.".to_string()
            }
            JoinByInviteLinkError::Unavailable(_) => {
                "This invite link has been revoked.".to_string()
            }
            JoinByInviteLinkError::CommunityNotFound => "Community not found".to_string(),
            JoinByInviteLinkError::Repository(e) => e.to_user_message(),
        }
    }
    fn to_error_code(&self) -> &'static str {
        match self {
            JoinByInviteLinkError::NotFound => "INVITE_NOT_FOUND",
            JoinByInviteLinkError::Unavailable(InviteLinkStatus::Expired) => "INVITE_EXPIRED",
            JoinByInviteLinkError::Unavailable(InviteLinkStatus::Exhausted) => "INVITE_EXHAUSTED",
            JoinByInviteLinkError::Unavailable(_) => "INVITE_REVOKED",
            JoinByInviteLinkError::CommunityNotFound => "COMMUNITY_NOT_FOUND",
            JoinByInviteLinkError::Repository(e) => e.to_error_code(),
        }
    }
}

impl HttpMapping for JoinVoiceChannelError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            JoinVoiceChannelError::ChannelNotFound | JoinVoiceChannelError::UserNotFound => {
                StatusCode::NOT_FOUND
            }
            JoinVoiceChannelError::PasswordRequired => StatusCode::BAD_REQUEST,
            JoinVoiceChannelError::InvalidPassword => StatusCode::FORBIDDEN,
            JoinVoiceChannelError::AccountInactive => StatusCode::FORBIDDEN,
            JoinVoiceChannelError::VoiceNotSupported => StatusCode::BAD_REQUEST,
            JoinVoiceChannelError::NotMember => StatusCode::FORBIDDEN,
            JoinVoiceChannelError::Repository(e) => e.to_http_status(),
            JoinVoiceChannelError::VoiceService(e) => e.to_http_status(),
            JoinVoiceChannelError::Hash(e) => e.to_http_status(),
            JoinVoiceChannelError::Domain(e) => e.to_http_status(),
        }
    }

    fn to_user_message(&self) -> String {
        match self {
            JoinVoiceChannelError::ChannelNotFound => "Channel not found".to_string(),
            JoinVoiceChannelError::UserNotFound => "User not found".to_string(),
            JoinVoiceChannelError::AccountInactive => "Account is inactive".to_string(),
            JoinVoiceChannelError::PasswordRequired => {
                "Password is required to join this channel".to_string()
            }
            JoinVoiceChannelError::InvalidPassword => "Invalid password".to_string(),
            JoinVoiceChannelError::VoiceNotSupported => {
                "Voice is not supported for this channel".to_string()
            }
            JoinVoiceChannelError::NotMember => "You are not a member of this community".to_string(),
            JoinVoiceChannelError::Repository(e) => e.to_user_message(),
            JoinVoiceChannelError::VoiceService(e) => e.to_user_message(),
            JoinVoiceChannelError::Hash(e) => e.to_user_message(),
            JoinVoiceChannelError::Domain(e) => e.to_user_message(),
        }
    }

    fn to_error_code(&self) -> &'static str {
        match self {
            JoinVoiceChannelError::ChannelNotFound => "CHANNEL_NOT_FOUND",
            JoinVoiceChannelError::UserNotFound => "USER_NOT_FOUND",
            JoinVoiceChannelError::AccountInactive => "ACCOUNT_INACTIVE",
            JoinVoiceChannelError::PasswordRequired => "CHANNEL_PASSWORD_REQUIRED",
            JoinVoiceChannelError::InvalidPassword => "INVALID_CHANNEL_PASSWORD",
            JoinVoiceChannelError::VoiceNotSupported => "VOICE_NOT_SUPPORTED",
            JoinVoiceChannelError::NotMember => "COMMUNITY_NOT_MEMBER",
            JoinVoiceChannelError::Repository(e) => e.to_error_code(),
            JoinVoiceChannelError::VoiceService(e) => e.to_error_code(),
            JoinVoiceChannelError::Hash(e) => e.to_error_code(),
            JoinVoiceChannelError::Domain(e) => e.to_error_code(),
        }
    }
}

impl HttpMapping for LogCallError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            LogCallError::Repository(e) => e.to_http_status(),
            LogCallError::Domain(e) => e.to_http_status(),
            LogCallError::SelfCall => StatusCode::BAD_REQUEST,
            LogCallError::CallNotVerified => StatusCode::FORBIDDEN,
            LogCallError::AlreadyLogged => StatusCode::CONFLICT,
            LogCallError::Lookup(_) => StatusCode::SERVICE_UNAVAILABLE,
        }
    }
    fn to_user_message(&self) -> String {
        match self {
            LogCallError::Repository(e) => e.to_user_message(),
            LogCallError::Domain(e) => e.to_user_message(),
            LogCallError::SelfCall => "You cannot log a call with yourself".to_string(),
            LogCallError::CallNotVerified => "Call could not be verified".to_string(),
            LogCallError::AlreadyLogged => "This call is already logged".to_string(),
            LogCallError::Lookup(_) => "Call verification unavailable".to_string(),
        }
    }
    fn to_error_code(&self) -> &'static str {
        match self {
            LogCallError::Repository(e) => e.to_error_code(),
            LogCallError::Domain(e) => e.to_error_code(),
            LogCallError::SelfCall => "SELF_CALL",
            LogCallError::CallNotVerified => "CALL_NOT_VERIFIED",
            LogCallError::AlreadyLogged => "CALL_ALREADY_LOGGED",
            LogCallError::Lookup(_) => "CALL_LOOKUP_UNAVAILABLE",
        }
    }
}

impl_http_mapping_delegate!(ListCommunitiesByUserError, Repository);
impl_http_mapping_delegate!(GetHomeOverviewError, Repository);
impl_http_mapping_delegate!(GetPrivacyError, Repository);

impl HttpMapping for ListCommunityMembersError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            ListCommunityMembersError::Repository(e) => e.to_http_status(),
            ListCommunityMembersError::NotMember => StatusCode::FORBIDDEN,
        }
    }

    fn to_user_message(&self) -> String {
        match self {
            ListCommunityMembersError::Repository(e) => e.to_user_message(),
            ListCommunityMembersError::NotMember => {
                "You are not a member of this community".to_string()
            }
        }
    }

    fn to_error_code(&self) -> &'static str {
        match self {
            ListCommunityMembersError::Repository(e) => e.to_error_code(),
            ListCommunityMembersError::NotMember => "COMMUNITY_NOT_MEMBER",
        }
    }
}

impl HttpMapping for GetVoicePresenceError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            GetVoicePresenceError::Repository(e) => e.to_http_status(),
            GetVoicePresenceError::NotMember => StatusCode::FORBIDDEN,
        }
    }

    fn to_user_message(&self) -> String {
        match self {
            GetVoicePresenceError::Repository(e) => e.to_user_message(),
            GetVoicePresenceError::NotMember => {
                "You are not a member of this community".to_string()
            }
        }
    }

    fn to_error_code(&self) -> &'static str {
        match self {
            GetVoicePresenceError::Repository(e) => e.to_error_code(),
            GetVoicePresenceError::NotMember => "COMMUNITY_NOT_MEMBER",
        }
    }
}

impl HttpMapping for UpdateStreamViewerError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            UpdateStreamViewerError::Repository(e) => e.to_http_status(),
            UpdateStreamViewerError::NotMember => StatusCode::FORBIDDEN,
        }
    }

    fn to_user_message(&self) -> String {
        match self {
            UpdateStreamViewerError::Repository(e) => e.to_user_message(),
            UpdateStreamViewerError::NotMember => {
                "You are not a member of this community".to_string()
            }
        }
    }

    fn to_error_code(&self) -> &'static str {
        match self {
            UpdateStreamViewerError::Repository(e) => e.to_error_code(),
            UpdateStreamViewerError::NotMember => "COMMUNITY_NOT_MEMBER",
        }
    }
}

impl HttpMapping for ListCommunityChannelsError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            ListCommunityChannelsError::Repository(e) => e.to_http_status(),
            ListCommunityChannelsError::NotMember => StatusCode::FORBIDDEN,
        }
    }

    fn to_user_message(&self) -> String {
        match self {
            ListCommunityChannelsError::Repository(e) => e.to_user_message(),
            ListCommunityChannelsError::NotMember => {
                "You are not a member of this community".to_string()
            }
        }
    }

    fn to_error_code(&self) -> &'static str {
        match self {
            ListCommunityChannelsError::Repository(e) => e.to_error_code(),
            ListCommunityChannelsError::NotMember => "COMMUNITY_NOT_MEMBER",
        }
    }
}

impl HttpMapping for JoinTextChannelError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            JoinTextChannelError::Repository(e) => e.to_http_status(),
            JoinTextChannelError::Domain(e) => e.to_http_status(),
            JoinTextChannelError::Hash(e) => e.to_http_status(),
            JoinTextChannelError::NotCommunityMember => StatusCode::FORBIDDEN,
            JoinTextChannelError::ChannelNotFound => StatusCode::NOT_FOUND,
            JoinTextChannelError::NotTextChannel => StatusCode::BAD_REQUEST,
            JoinTextChannelError::PasswordRequired => StatusCode::BAD_REQUEST,
            JoinTextChannelError::InvalidPassword => StatusCode::FORBIDDEN,
        }
    }

    fn to_user_message(&self) -> String {
        match self {
            JoinTextChannelError::Repository(e) => e.to_user_message(),
            JoinTextChannelError::Domain(e) => e.to_user_message(),
            JoinTextChannelError::Hash(e) => e.to_user_message(),
            JoinTextChannelError::NotCommunityMember => {
                "You are not a member of this community".to_string()
            }
            JoinTextChannelError::ChannelNotFound => "Channel not found".to_string(),
            JoinTextChannelError::NotTextChannel => "Channel does not support text".to_string(),
            JoinTextChannelError::PasswordRequired => {
                "Password is required to join this channel".to_string()
            }
            JoinTextChannelError::InvalidPassword => "Invalid password".to_string(),
        }
    }

    fn to_error_code(&self) -> &'static str {
        match self {
            JoinTextChannelError::Repository(e) => e.to_error_code(),
            JoinTextChannelError::Domain(e) => e.to_error_code(),
            JoinTextChannelError::Hash(e) => e.to_error_code(),
            JoinTextChannelError::NotCommunityMember => "COMMUNITY_NOT_MEMBER",
            JoinTextChannelError::ChannelNotFound => "CHANNEL_NOT_FOUND",
            JoinTextChannelError::NotTextChannel => "CHANNEL_NOT_TEXT",
            JoinTextChannelError::PasswordRequired => "CHANNEL_PASSWORD_REQUIRED",
            JoinTextChannelError::InvalidPassword => "INVALID_CHANNEL_PASSWORD",
        }
    }
}

impl HttpMapping for SendChannelMessageError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            SendChannelMessageError::Repository(e) => e.to_http_status(),
            SendChannelMessageError::Domain(e) => e.to_http_status(),
            SendChannelMessageError::Channel(e) => e.to_http_status(),
            SendChannelMessageError::NotCommunityMember
            | SendChannelMessageError::NotChannelMember => StatusCode::FORBIDDEN,
            SendChannelMessageError::ChannelNotFound => StatusCode::NOT_FOUND,
            SendChannelMessageError::NotTextChannel => StatusCode::BAD_REQUEST,
            SendChannelMessageError::InvalidAttachments => StatusCode::BAD_REQUEST,
            SendChannelMessageError::AttachmentsUnavailable(_) => StatusCode::SERVICE_UNAVAILABLE,
            SendChannelMessageError::InvalidReply => StatusCode::BAD_REQUEST,
        }
    }

    fn to_user_message(&self) -> String {
        match self {
            SendChannelMessageError::Repository(e) => e.to_user_message(),
            SendChannelMessageError::Domain(e) => e.to_user_message(),
            SendChannelMessageError::Channel(e) => e.to_user_message(),
            SendChannelMessageError::NotCommunityMember => {
                "You are not a member of this community".to_string()
            }
            SendChannelMessageError::NotChannelMember => {
                "You are not a member of this channel".to_string()
            }
            SendChannelMessageError::ChannelNotFound => "Channel not found".to_string(),
            SendChannelMessageError::InvalidAttachments => {
                "One or more attachments are invalid.".to_string()
            }
            SendChannelMessageError::AttachmentsUnavailable(_) => {
                "Attachment service is temporarily unavailable. Try again later.".to_string()
            }
            SendChannelMessageError::NotTextChannel => {
                "Channel does not support text".to_string()
            }
            SendChannelMessageError::InvalidReply => {
                "The message you are replying to is not available.".to_string()
            }
        }
    }

    fn to_error_code(&self) -> &'static str {
        match self {
            SendChannelMessageError::Repository(e) => e.to_error_code(),
            SendChannelMessageError::Domain(e) => e.to_error_code(),
            SendChannelMessageError::Channel(e) => e.to_error_code(),
            SendChannelMessageError::NotCommunityMember => "COMMUNITY_NOT_MEMBER",
            SendChannelMessageError::NotChannelMember => "CHANNEL_NOT_MEMBER",
            SendChannelMessageError::ChannelNotFound => "CHANNEL_NOT_FOUND",
            SendChannelMessageError::InvalidAttachments => "ATTACHMENTS_INVALID",
            SendChannelMessageError::AttachmentsUnavailable(_) => "ATTACHMENTS_UNAVAILABLE",
            SendChannelMessageError::NotTextChannel => "CHANNEL_NOT_TEXT",
            SendChannelMessageError::InvalidReply => "REPLY_INVALID",
        }
    }
}

impl HttpMapping for EditChannelMessageError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            EditChannelMessageError::Repository(e) => e.to_http_status(),
            EditChannelMessageError::Domain(e) => e.to_http_status(),
            EditChannelMessageError::NotCommunityMember => StatusCode::FORBIDDEN,
            EditChannelMessageError::MessageNotFound => StatusCode::NOT_FOUND,
            EditChannelMessageError::InvalidAttachments => StatusCode::BAD_REQUEST,
            EditChannelMessageError::AttachmentsUnavailable(_) => StatusCode::SERVICE_UNAVAILABLE,
        }
    }

    fn to_user_message(&self) -> String {
        match self {
            EditChannelMessageError::Repository(e) => e.to_user_message(),
            EditChannelMessageError::Domain(e) => e.to_user_message(),
            EditChannelMessageError::NotCommunityMember => {
                "You are not a member of this community".to_string()
            }
            EditChannelMessageError::MessageNotFound => "Message not found".to_string(),
            EditChannelMessageError::InvalidAttachments => {
                "Attachments are invalid or unavailable".to_string()
            }
            EditChannelMessageError::AttachmentsUnavailable(_) => {
                "Attachment service is temporarily unavailable. Try again later.".to_string()
            }
        }
    }

    fn to_error_code(&self) -> &'static str {
        match self {
            EditChannelMessageError::Repository(e) => e.to_error_code(),
            EditChannelMessageError::Domain(e) => e.to_error_code(),
            EditChannelMessageError::NotCommunityMember => "COMMUNITY_NOT_MEMBER",
            EditChannelMessageError::MessageNotFound => "MESSAGE_NOT_FOUND",
            EditChannelMessageError::InvalidAttachments => "ATTACHMENTS_INVALID",
            EditChannelMessageError::AttachmentsUnavailable(_) => "ATTACHMENTS_UNAVAILABLE",
        }
    }
}

impl HttpMapping for DeleteChannelMessageError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            DeleteChannelMessageError::Repository(e) => e.to_http_status(),
            DeleteChannelMessageError::Domain(e) => e.to_http_status(),
            DeleteChannelMessageError::NotCommunityMember => StatusCode::FORBIDDEN,
            DeleteChannelMessageError::MessageNotFound => StatusCode::NOT_FOUND,
        }
    }

    fn to_user_message(&self) -> String {
        match self {
            DeleteChannelMessageError::Repository(e) => e.to_user_message(),
            DeleteChannelMessageError::Domain(e) => e.to_user_message(),
            DeleteChannelMessageError::NotCommunityMember => {
                "You are not a member of this community".to_string()
            }
            DeleteChannelMessageError::MessageNotFound => "Message not found".to_string(),
        }
    }

    fn to_error_code(&self) -> &'static str {
        match self {
            DeleteChannelMessageError::Repository(e) => e.to_error_code(),
            DeleteChannelMessageError::Domain(e) => e.to_error_code(),
            DeleteChannelMessageError::NotCommunityMember => "COMMUNITY_NOT_MEMBER",
            DeleteChannelMessageError::MessageNotFound => "MESSAGE_NOT_FOUND",
        }
    }
}

impl HttpMapping for ListChannelMessagesError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            ListChannelMessagesError::Repository(e) => e.to_http_status(),
            ListChannelMessagesError::NotCommunityMember => StatusCode::FORBIDDEN,
            ListChannelMessagesError::NotChannelMember => StatusCode::FORBIDDEN,
            ListChannelMessagesError::ChannelNotFound => StatusCode::NOT_FOUND,
            ListChannelMessagesError::NotTextChannel => StatusCode::BAD_REQUEST,
            ListChannelMessagesError::InvalidCursor => StatusCode::BAD_REQUEST,
            ListChannelMessagesError::ConflictingCursors => StatusCode::BAD_REQUEST,
            ListChannelMessagesError::MessageNotFound => StatusCode::NOT_FOUND,
        }
    }

    fn to_user_message(&self) -> String {
        match self {
            ListChannelMessagesError::Repository(e) => e.to_user_message(),
            ListChannelMessagesError::NotCommunityMember => {
                "You are not a member of this community".to_string()
            }
            ListChannelMessagesError::NotChannelMember => {
                "You are not a member of this channel".to_string()
            }
            ListChannelMessagesError::ChannelNotFound => "Channel not found".to_string(),
            ListChannelMessagesError::NotTextChannel => {
                "Channel does not support text".to_string()
            }
            ListChannelMessagesError::InvalidCursor => "Invalid pagination cursor".to_string(),
            ListChannelMessagesError::ConflictingCursors => {
                "Cannot specify both before and after cursors".to_string()
            }
            ListChannelMessagesError::MessageNotFound => "Message not found".to_string(),
        }
    }

    fn to_error_code(&self) -> &'static str {
        match self {
            ListChannelMessagesError::Repository(e) => e.to_error_code(),
            ListChannelMessagesError::NotCommunityMember => "COMMUNITY_NOT_MEMBER",
            ListChannelMessagesError::NotChannelMember => "CHANNEL_NOT_MEMBER",
            ListChannelMessagesError::ChannelNotFound => "CHANNEL_NOT_FOUND",
            ListChannelMessagesError::NotTextChannel => "CHANNEL_NOT_TEXT",
            ListChannelMessagesError::InvalidCursor => "INVALID_CURSOR",
            ListChannelMessagesError::ConflictingCursors => "CONFLICTING_CURSORS",
            ListChannelMessagesError::MessageNotFound => "MESSAGE_NOT_FOUND",
        }
    }
}

impl HttpMapping for SearchChannelMessagesError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            SearchChannelMessagesError::Repository(e) => e.to_http_status(),
            SearchChannelMessagesError::NotCommunityMember => StatusCode::FORBIDDEN,
            SearchChannelMessagesError::NotChannelMember => StatusCode::FORBIDDEN,
            SearchChannelMessagesError::ChannelNotFound => StatusCode::NOT_FOUND,
            SearchChannelMessagesError::NotTextChannel => StatusCode::BAD_REQUEST,
            SearchChannelMessagesError::EmptyQuery => StatusCode::BAD_REQUEST,
        }
    }

    fn to_user_message(&self) -> String {
        match self {
            SearchChannelMessagesError::Repository(e) => e.to_user_message(),
            SearchChannelMessagesError::NotCommunityMember => {
                "You are not a member of this community".to_string()
            }
            SearchChannelMessagesError::NotChannelMember => {
                "You are not a member of this channel".to_string()
            }
            SearchChannelMessagesError::ChannelNotFound => "Channel not found".to_string(),
            SearchChannelMessagesError::NotTextChannel => {
                "This channel does not support text messages".to_string()
            }
            SearchChannelMessagesError::EmptyQuery => "Search query cannot be empty".to_string(),
        }
    }

    fn to_error_code(&self) -> &'static str {
        match self {
            SearchChannelMessagesError::Repository(e) => e.to_error_code(),
            SearchChannelMessagesError::NotCommunityMember => "COMMUNITY_NOT_MEMBER",
            SearchChannelMessagesError::NotChannelMember => "CHANNEL_NOT_MEMBER",
            SearchChannelMessagesError::ChannelNotFound => "CHANNEL_NOT_FOUND",
            SearchChannelMessagesError::NotTextChannel => "CHANNEL_NOT_TEXT",
            SearchChannelMessagesError::EmptyQuery => "EMPTY_QUERY",
        }
    }
}

impl HttpMapping for SendFriendRequestError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            SendFriendRequestError::Repository(e) => e.to_http_status(),
            SendFriendRequestError::Domain(e) => e.to_http_status(),
            SendFriendRequestError::InvalidUsername => StatusCode::BAD_REQUEST,
            SendFriendRequestError::UserNotFound => StatusCode::NOT_FOUND,
            SendFriendRequestError::SelfTarget => StatusCode::BAD_REQUEST,
            SendFriendRequestError::AlreadyRequested => StatusCode::CONFLICT,
            SendFriendRequestError::AlreadyFriends => StatusCode::CONFLICT,
            SendFriendRequestError::RequestsClosed => StatusCode::FORBIDDEN,
            SendFriendRequestError::Blocked => StatusCode::FORBIDDEN,
            SendFriendRequestError::IncomingPending => StatusCode::CONFLICT,
        }
    }

    fn to_user_message(&self) -> String {
        match self {
            SendFriendRequestError::Repository(e) => e.to_user_message(),
            SendFriendRequestError::Domain(e) => e.to_user_message(),
            SendFriendRequestError::InvalidUsername => "Invalid username".to_string(),
            SendFriendRequestError::UserNotFound => "User not found".to_string(),
            SendFriendRequestError::SelfTarget => {
                "You cannot send a friend request to yourself".to_string()
            }
            SendFriendRequestError::AlreadyRequested => {
                "Friend request already sent".to_string()
            }
            SendFriendRequestError::Blocked => "You cannot interact with this user.".to_string(),
            SendFriendRequestError::RequestsClosed => {
                "This user does not accept friend requests".to_string()
            }
            SendFriendRequestError::AlreadyFriends => {
                "You are already friends with this user".to_string()
            }
            SendFriendRequestError::IncomingPending => {
                "This user already sent you a friend request".to_string()
            }
        }
    }

    fn to_error_code(&self) -> &'static str {
        match self {
            SendFriendRequestError::Repository(e) => e.to_error_code(),
            SendFriendRequestError::Domain(e) => e.to_error_code(),
            SendFriendRequestError::InvalidUsername => "INVALID_USERNAME",
            SendFriendRequestError::UserNotFound => "USER_NOT_FOUND",
            SendFriendRequestError::SelfTarget => "SELF_FRIEND_REQUEST",
            SendFriendRequestError::AlreadyRequested => "FRIEND_REQUEST_ALREADY_SENT",
            SendFriendRequestError::AlreadyFriends => "ALREADY_FRIENDS",
            SendFriendRequestError::RequestsClosed => "REQUESTS_CLOSED",
            SendFriendRequestError::Blocked => "USER_BLOCKED",
            SendFriendRequestError::IncomingPending => "INCOMING_REQUEST_PENDING",
        }
    }
}

impl HttpMapping for AcceptFriendRequestError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            AcceptFriendRequestError::Repository(e) => e.to_http_status(),
            AcceptFriendRequestError::Domain(e) => e.to_http_status(),
            AcceptFriendRequestError::NoPendingRequest => StatusCode::NOT_FOUND,
            AcceptFriendRequestError::Blocked => StatusCode::FORBIDDEN,
        }
    }

    fn to_user_message(&self) -> String {
        match self {
            AcceptFriendRequestError::Repository(e) => e.to_user_message(),
            AcceptFriendRequestError::Domain(e) => e.to_user_message(),
            AcceptFriendRequestError::Blocked => "You cannot interact with this user.".to_string(),
            AcceptFriendRequestError::NoPendingRequest => {
                "No pending friend request from this user".to_string()
            }
        }
    }

    fn to_error_code(&self) -> &'static str {
        match self {
            AcceptFriendRequestError::Repository(e) => e.to_error_code(),
            AcceptFriendRequestError::Domain(e) => e.to_error_code(),
            AcceptFriendRequestError::NoPendingRequest => "NO_PENDING_REQUEST",
            AcceptFriendRequestError::Blocked => "USER_BLOCKED",
        }
    }
}

impl HttpMapping for RemoveFriendshipError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            RemoveFriendshipError::Repository(e) => e.to_http_status(),
            RemoveFriendshipError::NotFound => StatusCode::NOT_FOUND,
        }
    }

    fn to_user_message(&self) -> String {
        match self {
            RemoveFriendshipError::Repository(e) => e.to_user_message(),
            RemoveFriendshipError::NotFound => "Friendship not found".to_string(),
        }
    }

    fn to_error_code(&self) -> &'static str {
        match self {
            RemoveFriendshipError::Repository(e) => e.to_error_code(),
            RemoveFriendshipError::NotFound => "FRIENDSHIP_NOT_FOUND",
        }
    }
}

impl HttpMapping for SendDmMessageError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            SendDmMessageError::Repository(e) => e.to_http_status(),
            SendDmMessageError::Domain(e) => e.to_http_status(),
            SendDmMessageError::NotFriends => StatusCode::FORBIDDEN,
            SendDmMessageError::Blocked => StatusCode::FORBIDDEN,
            SendDmMessageError::DmsClosed => StatusCode::FORBIDDEN,
            SendDmMessageError::SelfDm => StatusCode::BAD_REQUEST,
            SendDmMessageError::RecipientInactive => StatusCode::FORBIDDEN,
            SendDmMessageError::InvalidAttachments => StatusCode::BAD_REQUEST,
            SendDmMessageError::AttachmentsUnavailable(_) => StatusCode::SERVICE_UNAVAILABLE,
            SendDmMessageError::InvalidReply => StatusCode::BAD_REQUEST,
        }
    }
    fn to_user_message(&self) -> String {
        match self {
            SendDmMessageError::Repository(e) => e.to_user_message(),
            SendDmMessageError::Domain(e) => e.to_user_message(),
            SendDmMessageError::InvalidAttachments => {
                "One or more attachments are invalid.".to_string()
            }
            SendDmMessageError::AttachmentsUnavailable(_) => {
                "Attachment service is temporarily unavailable. Try again later.".to_string()
            }
            SendDmMessageError::Blocked => "You cannot interact with this user.".to_string(),
            SendDmMessageError::NotFriends => {
                "You can only DM users who are your friends".to_string()
            }
            SendDmMessageError::DmsClosed => {
                "This user does not accept direct messages".to_string()
            }
            SendDmMessageError::SelfDm => "You cannot message yourself".to_string(),
            SendDmMessageError::RecipientInactive => "This user is unavailable".to_string(),
            SendDmMessageError::InvalidReply => {
                "The message you are replying to is not available.".to_string()
            }
        }
    }
    fn to_error_code(&self) -> &'static str {
        match self {
            SendDmMessageError::Repository(e) => e.to_error_code(),
            SendDmMessageError::Domain(e) => e.to_error_code(),
            SendDmMessageError::InvalidAttachments => "ATTACHMENTS_INVALID",
            SendDmMessageError::AttachmentsUnavailable(_) => "ATTACHMENTS_UNAVAILABLE",
            SendDmMessageError::NotFriends => "NOT_FRIENDS",
            SendDmMessageError::Blocked => "USER_BLOCKED",
            SendDmMessageError::DmsClosed => "DMS_CLOSED",
            SendDmMessageError::SelfDm => "SELF_DM",
            SendDmMessageError::RecipientInactive => "RECIPIENT_INACTIVE",
            SendDmMessageError::InvalidReply => "REPLY_INVALID",
        }
    }
}

impl HttpMapping for SendCommunityInviteError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            SendCommunityInviteError::Repository(e) => e.to_http_status(),
            SendCommunityInviteError::Domain(e) => e.to_http_status(),
            SendCommunityInviteError::NotFriends => StatusCode::FORBIDDEN,
            SendCommunityInviteError::Blocked => StatusCode::FORBIDDEN,
            SendCommunityInviteError::DmsClosed => StatusCode::FORBIDDEN,
            SendCommunityInviteError::RecipientInactive => StatusCode::FORBIDDEN,
            SendCommunityInviteError::CommunityNotFound => StatusCode::NOT_FOUND,
            SendCommunityInviteError::NotCommunityMember => StatusCode::FORBIDDEN,
            SendCommunityInviteError::NotAllowed => StatusCode::FORBIDDEN,
            SendCommunityInviteError::AlreadyMember => StatusCode::CONFLICT,
            SendCommunityInviteError::InviteAlreadyPending => StatusCode::CONFLICT,
        }
    }
    fn to_user_message(&self) -> String {
        match self {
            SendCommunityInviteError::Repository(e) => e.to_user_message(),
            SendCommunityInviteError::Domain(e) => e.to_user_message(),
            SendCommunityInviteError::DmsClosed => {
                "This user does not accept direct messages".to_string()
            }
            SendCommunityInviteError::Blocked => "You cannot interact with this user.".to_string(),
            SendCommunityInviteError::NotFriends => {
                "You can only invite users who are your friends".to_string()
            }
            SendCommunityInviteError::RecipientInactive => "This user is unavailable".to_string(),
            SendCommunityInviteError::CommunityNotFound => "Community not found".to_string(),
            SendCommunityInviteError::NotCommunityMember => {
                "Only members of this community can invite people".to_string()
            }
            SendCommunityInviteError::NotAllowed => {
                "You don't have permission to manage invites".to_string()
            }
            SendCommunityInviteError::AlreadyMember => {
                "This user is already a member of the community".to_string()
            }
            SendCommunityInviteError::InviteAlreadyPending => {
                "You've already invited this user to this community".to_string()
            }
        }
    }
    fn to_error_code(&self) -> &'static str {
        match self {
            SendCommunityInviteError::Repository(e) => e.to_error_code(),
            SendCommunityInviteError::Domain(e) => e.to_error_code(),
            SendCommunityInviteError::NotFriends => "NOT_FRIENDS",
            SendCommunityInviteError::Blocked => "USER_BLOCKED",
            SendCommunityInviteError::DmsClosed => "DMS_CLOSED",
            SendCommunityInviteError::RecipientInactive => "RECIPIENT_INACTIVE",
            SendCommunityInviteError::CommunityNotFound => "COMMUNITY_NOT_FOUND",
            SendCommunityInviteError::NotCommunityMember => "COMMUNITY_NOT_MEMBER",
            SendCommunityInviteError::NotAllowed => "INVITE_NOT_ALLOWED",
            SendCommunityInviteError::AlreadyMember => "ALREADY_MEMBER",
            SendCommunityInviteError::InviteAlreadyPending => "INVITE_ALREADY_PENDING",
        }
    }
}

impl HttpMapping for KickMemberError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            KickMemberError::Repository(e) => e.to_http_status(),
            KickMemberError::NotMember => StatusCode::NOT_FOUND,
            KickMemberError::NotAllowed => StatusCode::FORBIDDEN,
        }
    }

    fn to_user_message(&self) -> String {
        match self {
            KickMemberError::Repository(e) => e.to_user_message(),
            KickMemberError::NotMember => "User is not a member of this community".to_string(),
            KickMemberError::NotAllowed => {
                "You don't have permission to kick this member".to_string()
            }
        }
    }

    fn to_error_code(&self) -> &'static str {
        match self {
            KickMemberError::Repository(e) => e.to_error_code(),
            KickMemberError::NotMember => "COMMUNITY_NOT_MEMBER",
            KickMemberError::NotAllowed => "KICK_NOT_ALLOWED",
        }
    }
}

impl HttpMapping for LeaveCommunityError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            LeaveCommunityError::Repository(e) => e.to_http_status(),
            LeaveCommunityError::NotMember => StatusCode::NOT_FOUND,
            LeaveCommunityError::OwnerCannotLeave => StatusCode::CONFLICT,
        }
    }
    fn to_user_message(&self) -> String {
        match self {
            LeaveCommunityError::Repository(e) => e.to_user_message(),
            LeaveCommunityError::NotMember => {
                "You are not a member of this community".to_string()
            }
            LeaveCommunityError::OwnerCannotLeave => {
                "Community owners cannot leave their own community; delete it instead".to_string()
            }
        }
    }
    fn to_error_code(&self) -> &'static str {
        match self {
            LeaveCommunityError::Repository(e) => e.to_error_code(),
            LeaveCommunityError::NotMember => "COMMUNITY_NOT_MEMBER",
            LeaveCommunityError::OwnerCannotLeave => "COMMUNITY_OWNER_CANNOT_LEAVE",
        }
    }
}

impl HttpMapping for ListSentCommunityInvitesError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            ListSentCommunityInvitesError::Repository(e) => e.to_http_status(),
            ListSentCommunityInvitesError::NotMember => StatusCode::FORBIDDEN,
        }
    }
    fn to_user_message(&self) -> String {
        match self {
            ListSentCommunityInvitesError::Repository(e) => e.to_user_message(),
            ListSentCommunityInvitesError::NotMember => {
                "You are not a member of this community".to_string()
            }
        }
    }
    fn to_error_code(&self) -> &'static str {
        match self {
            ListSentCommunityInvitesError::Repository(e) => e.to_error_code(),
            ListSentCommunityInvitesError::NotMember => "COMMUNITY_NOT_MEMBER",
        }
    }
}

impl HttpMapping for DeclineCommunityInviteError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            DeclineCommunityInviteError::Repository(e) => e.to_http_status(),
            DeclineCommunityInviteError::NotAnInvite => StatusCode::BAD_REQUEST,
            DeclineCommunityInviteError::NotRecipient => StatusCode::FORBIDDEN,
        }
    }
    fn to_user_message(&self) -> String {
        match self {
            DeclineCommunityInviteError::Repository(e) => e.to_user_message(),
            DeclineCommunityInviteError::NotAnInvite => {
                "This message is not a community invite".to_string()
            }
            DeclineCommunityInviteError::NotRecipient => {
                "This invite was not addressed to you".to_string()
            }
        }
    }
    fn to_error_code(&self) -> &'static str {
        match self {
            DeclineCommunityInviteError::Repository(e) => e.to_error_code(),
            DeclineCommunityInviteError::NotAnInvite => "INVITE_INVALID",
            DeclineCommunityInviteError::NotRecipient => "INVITE_NOT_RECIPIENT",
        }
    }
}

impl HttpMapping for AcceptCommunityInviteError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            AcceptCommunityInviteError::Repository(e) => e.to_http_status(),
            AcceptCommunityInviteError::NotAnInvite => StatusCode::BAD_REQUEST,
            AcceptCommunityInviteError::NotRecipient => StatusCode::FORBIDDEN,
            AcceptCommunityInviteError::Blocked => StatusCode::FORBIDDEN,
        }
    }
    fn to_user_message(&self) -> String {
        match self {
            AcceptCommunityInviteError::Repository(e) => e.to_user_message(),
            AcceptCommunityInviteError::NotAnInvite => {
                "This message is not a community invite".to_string()
            }
            AcceptCommunityInviteError::NotRecipient => {
                "This invite was not addressed to you".to_string()
            }
            AcceptCommunityInviteError::Blocked => "You cannot interact with this user.".to_string(),
        }
    }
    fn to_error_code(&self) -> &'static str {
        match self {
            AcceptCommunityInviteError::Repository(e) => e.to_error_code(),
            AcceptCommunityInviteError::NotAnInvite => "INVITE_INVALID",
            AcceptCommunityInviteError::NotRecipient => "INVITE_NOT_RECIPIENT",
            AcceptCommunityInviteError::Blocked => "USER_BLOCKED",
        }
    }
}

impl HttpMapping for EditDmMessageError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            EditDmMessageError::Repository(e) => e.to_http_status(),
            EditDmMessageError::Dm(e) => e.to_http_status(),
            EditDmMessageError::Message(e) => e.to_http_status(),
            EditDmMessageError::NotFound => StatusCode::NOT_FOUND,
            EditDmMessageError::NotFriends => StatusCode::FORBIDDEN,
            EditDmMessageError::Blocked => StatusCode::FORBIDDEN,
            EditDmMessageError::DmsClosed => StatusCode::FORBIDDEN,
            EditDmMessageError::RecipientInactive => StatusCode::FORBIDDEN,
            EditDmMessageError::InvalidAttachments => StatusCode::BAD_REQUEST,
            EditDmMessageError::AttachmentsUnavailable(_) => StatusCode::SERVICE_UNAVAILABLE,
        }
    }
    fn to_user_message(&self) -> String {
        match self {
            EditDmMessageError::Repository(e) => e.to_user_message(),
            EditDmMessageError::Dm(e) => e.to_user_message(),
            EditDmMessageError::Message(e) => e.to_user_message(),
            EditDmMessageError::NotFound => "Message not found".to_string(),
            EditDmMessageError::Blocked => "You cannot interact with this user.".to_string(),
            EditDmMessageError::NotFriends => {
                "You can only DM users who are your friends".to_string()
            }
            EditDmMessageError::DmsClosed => {
                "This user does not accept direct messages".to_string()
            }
            EditDmMessageError::RecipientInactive => "This user is unavailable".to_string(),
            EditDmMessageError::InvalidAttachments => {
                "Attachments are invalid or unavailable".to_string()
            }
            EditDmMessageError::AttachmentsUnavailable(_) => {
                "Attachment service is temporarily unavailable. Try again later.".to_string()
            }
        }
    }
    fn to_error_code(&self) -> &'static str {
        match self {
            EditDmMessageError::Repository(e) => e.to_error_code(),
            EditDmMessageError::Dm(e) => e.to_error_code(),
            EditDmMessageError::Message(e) => e.to_error_code(),
            EditDmMessageError::NotFound => "DM_NOT_FOUND",
            EditDmMessageError::NotFriends => "NOT_FRIENDS",
            EditDmMessageError::Blocked => "USER_BLOCKED",
            EditDmMessageError::DmsClosed => "DMS_CLOSED",
            EditDmMessageError::RecipientInactive => "RECIPIENT_INACTIVE",
            EditDmMessageError::InvalidAttachments => "ATTACHMENTS_INVALID",
            EditDmMessageError::AttachmentsUnavailable(_) => "ATTACHMENTS_UNAVAILABLE",
        }
    }
}

impl HttpMapping for DeleteDmMessageError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            DeleteDmMessageError::Repository(e) => e.to_http_status(),
            DeleteDmMessageError::Dm(e) => e.to_http_status(),
            DeleteDmMessageError::NotFound => StatusCode::NOT_FOUND,
        }
    }
    fn to_user_message(&self) -> String {
        match self {
            DeleteDmMessageError::Repository(e) => e.to_user_message(),
            DeleteDmMessageError::Dm(e) => e.to_user_message(),
            DeleteDmMessageError::NotFound => "Message not found".to_string(),
        }
    }
    fn to_error_code(&self) -> &'static str {
        match self {
            DeleteDmMessageError::Repository(e) => e.to_error_code(),
            DeleteDmMessageError::Dm(e) => e.to_error_code(),
            DeleteDmMessageError::NotFound => "DM_NOT_FOUND",
        }
    }
}

impl HttpMapping for GetAttachmentError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            GetAttachmentError::NotFound => StatusCode::NOT_FOUND,
            GetAttachmentError::Claims(_) => StatusCode::SERVICE_UNAVAILABLE,
            GetAttachmentError::Storage(e) => e.to_http_status(),
            GetAttachmentError::Repository(e) => e.to_http_status(),
        }
    }
    fn to_user_message(&self) -> String {
        match self {
            GetAttachmentError::NotFound => "Attachment not found.".to_string(),
            GetAttachmentError::Claims(_) => {
                "Attachment service is temporarily unavailable. Try again later.".to_string()
            }
            GetAttachmentError::Storage(e) => e.to_user_message(),
            GetAttachmentError::Repository(e) => e.to_user_message(),
        }
    }
    fn to_error_code(&self) -> &'static str {
        match self {
            GetAttachmentError::NotFound => "ATTACHMENT_NOT_FOUND",
            GetAttachmentError::Claims(_) => "ATTACHMENTS_UNAVAILABLE",
            GetAttachmentError::Storage(e) => e.to_error_code(),
            GetAttachmentError::Repository(e) => e.to_error_code(),
        }
    }
}

impl HttpMapping for ListMessageAttachmentsError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            ListMessageAttachmentsError::Repository(e) => e.to_http_status(),
        }
    }
    fn to_user_message(&self) -> String {
        match self {
            ListMessageAttachmentsError::Repository(e) => e.to_user_message(),
        }
    }
    fn to_error_code(&self) -> &'static str {
        match self {
            ListMessageAttachmentsError::Repository(e) => e.to_error_code(),
        }
    }
}

impl HttpMapping for ListDmPreviewsError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            ListDmPreviewsError::Repository(e) => e.to_http_status(),
        }
    }
    fn to_user_message(&self) -> String {
        match self {
            ListDmPreviewsError::Repository(e) => e.to_user_message(),
        }
    }
    fn to_error_code(&self) -> &'static str {
        match self {
            ListDmPreviewsError::Repository(e) => e.to_error_code(),
        }
    }
}

impl HttpMapping for ListDmMessagesError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            ListDmMessagesError::Repository(e) => e.to_http_status(),
            ListDmMessagesError::InvalidCursor
            | ListDmMessagesError::ConflictingCursors => StatusCode::BAD_REQUEST,
            ListDmMessagesError::MessageNotFound => StatusCode::NOT_FOUND,
        }
    }
    fn to_user_message(&self) -> String {
        match self {
            ListDmMessagesError::Repository(e) => e.to_user_message(),
            ListDmMessagesError::InvalidCursor => "Invalid pagination cursor".to_string(),
            ListDmMessagesError::ConflictingCursors => {
                "Cannot specify both before and after cursors".to_string()
            }
            ListDmMessagesError::MessageNotFound => "Message not found".to_string(),
        }
    }
    fn to_error_code(&self) -> &'static str {
        match self {
            ListDmMessagesError::Repository(e) => e.to_error_code(),
            ListDmMessagesError::InvalidCursor => "INVALID_CURSOR",
            ListDmMessagesError::ConflictingCursors => "CONFLICTING_CURSORS",
            ListDmMessagesError::MessageNotFound => "MESSAGE_NOT_FOUND",
        }
    }
}

impl HttpMapping for SearchDmMessagesError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            SearchDmMessagesError::Repository(e) => e.to_http_status(),
            SearchDmMessagesError::EmptyQuery => StatusCode::BAD_REQUEST,
        }
    }
    fn to_user_message(&self) -> String {
        match self {
            SearchDmMessagesError::Repository(e) => e.to_user_message(),
            SearchDmMessagesError::EmptyQuery => "Search query cannot be empty".to_string(),
        }
    }
    fn to_error_code(&self) -> &'static str {
        match self {
            SearchDmMessagesError::Repository(e) => e.to_error_code(),
            SearchDmMessagesError::EmptyQuery => "EMPTY_QUERY",
        }
    }
}

impl HttpMapping for MarkChannelReadError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            MarkChannelReadError::Repository(e) => e.to_http_status(),
            MarkChannelReadError::NotMember => StatusCode::FORBIDDEN,
        }
    }
    fn to_user_message(&self) -> String {
        match self {
            MarkChannelReadError::Repository(e) => e.to_user_message(),
            MarkChannelReadError::NotMember => {
                "You are not a member of this channel".to_string()
            }
        }
    }
    fn to_error_code(&self) -> &'static str {
        match self {
            MarkChannelReadError::Repository(e) => e.to_error_code(),
            MarkChannelReadError::NotMember => "CHANNEL_NOT_MEMBER",
        }
    }
}

impl HttpMapping for MarkDmReadError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            MarkDmReadError::Repository(e) => e.to_http_status(),
            MarkDmReadError::NoConversation => StatusCode::FORBIDDEN,
            MarkDmReadError::Blocked => StatusCode::FORBIDDEN,
        }
    }
    fn to_user_message(&self) -> String {
        match self {
            MarkDmReadError::Repository(e) => e.to_user_message(),
            MarkDmReadError::NoConversation => "No conversation with this user".to_string(),
            MarkDmReadError::Blocked => "You cannot interact with this user.".to_string(),
        }
    }
    fn to_error_code(&self) -> &'static str {
        match self {
            MarkDmReadError::Repository(e) => e.to_error_code(),
            MarkDmReadError::NoConversation => "NO_CONVERSATION",
            MarkDmReadError::Blocked => "USER_BLOCKED",
        }
    }
}

impl HttpMapping for GetUnreadCountsError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            GetUnreadCountsError::Repository(e) => e.to_http_status(),
        }
    }
    fn to_user_message(&self) -> String {
        match self {
            GetUnreadCountsError::Repository(e) => e.to_user_message(),
        }
    }
    fn to_error_code(&self) -> &'static str {
        match self {
            GetUnreadCountsError::Repository(e) => e.to_error_code(),
        }
    }
}

impl HttpMapping for GetDmReadReceiptError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            GetDmReadReceiptError::Repository(e) => e.to_http_status(),
            GetDmReadReceiptError::NoConversation => StatusCode::FORBIDDEN,
            GetDmReadReceiptError::Blocked => StatusCode::FORBIDDEN,
        }
    }
    fn to_user_message(&self) -> String {
        match self {
            GetDmReadReceiptError::Repository(e) => e.to_user_message(),
            GetDmReadReceiptError::NoConversation => "No conversation with this user".to_string(),
            GetDmReadReceiptError::Blocked => "You cannot interact with this user.".to_string(),
        }
    }
    fn to_error_code(&self) -> &'static str {
        match self {
            GetDmReadReceiptError::Repository(e) => e.to_error_code(),
            GetDmReadReceiptError::NoConversation => "NO_CONVERSATION",
            GetDmReadReceiptError::Blocked => "USER_BLOCKED",
        }
    }
}

impl HttpMapping for GetChannelReadReceiptsError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            GetChannelReadReceiptsError::Repository(e) => e.to_http_status(),
            GetChannelReadReceiptsError::NotMember => StatusCode::FORBIDDEN,
        }
    }
    fn to_user_message(&self) -> String {
        match self {
            GetChannelReadReceiptsError::Repository(e) => e.to_user_message(),
            GetChannelReadReceiptsError::NotMember => {
                "You are not a member of this channel".to_string()
            }
        }
    }
    fn to_error_code(&self) -> &'static str {
        match self {
            GetChannelReadReceiptsError::Repository(e) => e.to_error_code(),
            GetChannelReadReceiptsError::NotMember => "CHANNEL_NOT_MEMBER",
        }
    }
}

impl HttpMapping for ListFriendsError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            ListFriendsError::Repository(e) => e.to_http_status(),
        }
    }

    fn to_user_message(&self) -> String {
        match self {
            ListFriendsError::Repository(e) => e.to_user_message(),
        }
    }

    fn to_error_code(&self) -> &'static str {
        match self {
            ListFriendsError::Repository(e) => e.to_error_code(),
        }
    }
}

impl HttpMapping for ListChannelParticipantsError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            ListChannelParticipantsError::Repository(e) => e.to_http_status(),
            ListChannelParticipantsError::NotMember => StatusCode::FORBIDDEN,
            ListChannelParticipantsError::NotChannelMember => StatusCode::FORBIDDEN,
            ListChannelParticipantsError::ChannelNotFound => StatusCode::NOT_FOUND,
        }
    }

    fn to_user_message(&self) -> String {
        match self {
            ListChannelParticipantsError::Repository(e) => e.to_user_message(),
            ListChannelParticipantsError::NotMember => {
                "You are not a member of this community".to_string()
            }
            ListChannelParticipantsError::NotChannelMember => {
                "You are not a member of this channel".to_string()
            }
            ListChannelParticipantsError::ChannelNotFound => "Channel not found".to_string(),
        }
    }

    fn to_error_code(&self) -> &'static str {
        match self {
            ListChannelParticipantsError::Repository(e) => e.to_error_code(),
            ListChannelParticipantsError::NotMember => "COMMUNITY_NOT_MEMBER",
            ListChannelParticipantsError::NotChannelMember => "CHANNEL_NOT_MEMBER",
            ListChannelParticipantsError::ChannelNotFound => "CHANNEL_NOT_FOUND",
        }
    }
}

impl HttpMapping for RoleError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            RoleError::Repository(e) => e.to_http_status(),
            RoleError::NotMember => StatusCode::FORBIDDEN,
            RoleError::NotAllowed => StatusCode::FORBIDDEN,
            RoleError::HierarchyViolation => StatusCode::FORBIDDEN,
            RoleError::RoleNotFound => StatusCode::NOT_FOUND,
            RoleError::InvalidData => StatusCode::BAD_REQUEST,
        }
    }
    fn to_user_message(&self) -> String {
        match self {
            RoleError::Repository(e) => e.to_user_message(),
            RoleError::NotMember => "You are not a member of this community".to_string(),
            RoleError::NotAllowed => {
                "You don't have permission to manage roles".to_string()
            }
            RoleError::HierarchyViolation => {
                "You can only manage roles below your highest role".to_string()
            }
            RoleError::RoleNotFound => "Role not found".to_string(),
            RoleError::InvalidData => "Invalid role data".to_string(),
        }
    }
    fn to_error_code(&self) -> &'static str {
        match self {
            RoleError::Repository(e) => e.to_error_code(),
            RoleError::NotMember => "COMMUNITY_NOT_MEMBER",
            RoleError::NotAllowed => "ROLE_NOT_ALLOWED",
            RoleError::HierarchyViolation => "ROLE_HIERARCHY",
            RoleError::RoleNotFound => "ROLE_NOT_FOUND",
            RoleError::InvalidData => "ROLE_INVALID",
        }
    }
}

impl HttpMapping for SetNotificationPreferenceError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            SetNotificationPreferenceError::Repository(e) => e.to_http_status(),
            SetNotificationPreferenceError::NotMember => StatusCode::FORBIDDEN,
            SetNotificationPreferenceError::NotFriends => StatusCode::FORBIDDEN,
            SetNotificationPreferenceError::UnsupportedLevel => StatusCode::BAD_REQUEST,
        }
    }
    fn to_user_message(&self) -> String {
        match self {
            SetNotificationPreferenceError::Repository(e) => e.to_user_message(),
            SetNotificationPreferenceError::NotMember => {
                "You are not a member of this channel".to_string()
            }
            SetNotificationPreferenceError::NotFriends => {
                "You are not friends with this user".to_string()
            }
            SetNotificationPreferenceError::UnsupportedLevel => {
                "This notification level is not supported for this scope".to_string()
            }
        }
    }
    fn to_error_code(&self) -> &'static str {
        match self {
            SetNotificationPreferenceError::Repository(e) => e.to_error_code(),
            SetNotificationPreferenceError::NotMember => "CHANNEL_NOT_MEMBER",
            SetNotificationPreferenceError::NotFriends => "NOT_FRIENDS",
            SetNotificationPreferenceError::UnsupportedLevel => "UNSUPPORTED_LEVEL",
        }
    }
}

impl_http_mapping_delegate!(ListNotificationPreferencesError, Repository);


impl HttpMapping for ChannelError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            ChannelError::PrivateChannelRequiresPassword
            | ChannelError::PublicChannelCannotHavePassword
            | ChannelError::NameUnchanged => StatusCode::BAD_REQUEST,
            ChannelError::NotAllowed => StatusCode::FORBIDDEN,
            ChannelError::AlreadyMuted | ChannelError::NotMuted => StatusCode::CONFLICT,
            ChannelError::AlreadyDeafened | ChannelError::NotDeafened => StatusCode::CONFLICT,
            ChannelError::VoiceNotSupported | ChannelError::TextNotSupported => {
                StatusCode::BAD_REQUEST
            }
            ChannelError::ParticipantNotJoined => StatusCode::CONFLICT,
            ChannelError::DefaultChannel => StatusCode::CONFLICT,
        }
    }

    fn to_user_message(&self) -> String {
        match self {
            ChannelError::PrivateChannelRequiresPassword => {
                "Private channel requires a password".to_string()
            }
            ChannelError::PublicChannelCannotHavePassword => {
                "Public channel cannot have a password".to_string()
            }
            ChannelError::NameUnchanged => "Channel name is unchanged".to_string(),
            ChannelError::NotAllowed => "You are not allowed to perform this channel action".to_string(),
            ChannelError::AlreadyMuted => "Participant is already muted".to_string(),
            ChannelError::NotMuted => "Participant is not muted".to_string(),
            ChannelError::AlreadyDeafened => "Participant is already deafened".to_string(),
            ChannelError::NotDeafened => "Participant is not deafened".to_string(),
            ChannelError::VoiceNotSupported => {
                "Voice is not supported for this channel".to_string()
            }
            ChannelError::TextNotSupported => {
                "Text is not supported for this channel".to_string()
            }
            ChannelError::ParticipantNotJoined => {
                "Participant has not joined the voice session yet".to_string()
            }
            ChannelError::DefaultChannel => "The default channel cannot be deleted".to_string(),
        }
    }

    fn to_error_code(&self) -> &'static str {
        match self {
            ChannelError::PrivateChannelRequiresPassword => "CHANNEL_PASSWORD_REQUIRED",
            ChannelError::PublicChannelCannotHavePassword => "CHANNEL_PASSWORD_NOT_ALLOWED",
            ChannelError::NameUnchanged => "CHANNEL_NAME_UNCHANGED",
            ChannelError::NotAllowed => "CHANNEL_NOT_ALLOWED",
            ChannelError::AlreadyMuted => "PARTICIPANT_ALREADY_MUTED",
            ChannelError::NotMuted => "PARTICIPANT_NOT_MUTED",
            ChannelError::AlreadyDeafened => "PARTICIPANT_ALREADY_DEAFENED",
            ChannelError::NotDeafened => "PARTICIPANT_NOT_DEAFENED",
            ChannelError::VoiceNotSupported => "VOICE_NOT_SUPPORTED",
            ChannelError::TextNotSupported => "CHANNEL_NOT_TEXT",
            ChannelError::ParticipantNotJoined => "PARTICIPANT_NOT_JOINED",
            ChannelError::DefaultChannel => "CHANNEL_IS_DEFAULT",
        }
    }
}

impl HttpMapping for MessageError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            MessageError::ContentEmpty
            | MessageError::ContentTooLong
            | MessageError::ContentUnchanged => StatusCode::BAD_REQUEST,
            MessageError::NotAuthor => StatusCode::FORBIDDEN,
            MessageError::SystemMessage => StatusCode::CONFLICT,
        }
    }

    fn to_user_message(&self) -> String {
        match self {
            MessageError::ContentEmpty => "Message content cannot be empty".to_string(),
            MessageError::ContentTooLong => {
                "Message content exceeds maximum length".to_string()
            }
            MessageError::ContentUnchanged => "Message content unchanged".to_string(),
            MessageError::NotAuthor => {
                "Only the message author can perform this action".to_string()
            }
            MessageError::SystemMessage => "System messages cannot be edited or deleted".to_string(),
        }
    }

    fn to_error_code(&self) -> &'static str {
        match self {
            MessageError::ContentEmpty => "MESSAGE_CONTENT_EMPTY",
            MessageError::ContentTooLong => "MESSAGE_CONTENT_TOO_LONG",
            MessageError::ContentUnchanged => "MESSAGE_CONTENT_UNCHANGED",
            MessageError::NotAuthor => "MESSAGE_NOT_AUTHOR",
            MessageError::SystemMessage => "MESSAGE_IS_SYSTEM",
        }
    }
}

impl HttpMapping for FriendshipError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            FriendshipError::SelfFriendship => StatusCode::BAD_REQUEST,
            FriendshipError::AlreadyRequested
            | FriendshipError::IncomingPending
            | FriendshipError::AlreadyFriends => StatusCode::CONFLICT,
            FriendshipError::NoPendingRequest | FriendshipError::NotFriends => {
                StatusCode::NOT_FOUND
            }
        }
    }

    fn to_user_message(&self) -> String {
        self.to_string()
    }

    fn to_error_code(&self) -> &'static str {
        match self {
            FriendshipError::SelfFriendship => "FRIENDSHIP_SELF",
            FriendshipError::AlreadyRequested => "FRIEND_REQUEST_ALREADY_SENT",
            FriendshipError::IncomingPending => "FRIEND_REQUEST_INCOMING_PENDING",
            FriendshipError::AlreadyFriends => "ALREADY_FRIENDS",
            FriendshipError::NoPendingRequest => "NO_PENDING_REQUEST",
            FriendshipError::NotFriends => "NOT_FRIENDS",
        }
    }
}

impl HttpMapping for DmMessageError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            DmMessageError::NotSender => StatusCode::FORBIDDEN,
            DmMessageError::ContentUnchanged => StatusCode::BAD_REQUEST,
            DmMessageError::NotEditable => StatusCode::BAD_REQUEST,
        }
    }
    fn to_user_message(&self) -> String {
        self.to_string()
    }
    fn to_error_code(&self) -> &'static str {
        match self {
            DmMessageError::NotSender => "DM_NOT_SENDER",
            DmMessageError::ContentUnchanged => "DM_CONTENT_UNCHANGED",
            DmMessageError::NotEditable => "DM_NOT_EDITABLE",
        }
    }
}

impl HttpMapping for ports::voice_service::VoiceServiceError {
    fn to_http_status(&self) -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }

    fn to_user_message(&self) -> String {
        "Voice service error".to_string()
    }

    fn to_error_code(&self) -> &'static str {
        match self {
            ports::voice_service::VoiceServiceError::TokenCreation(_) => "VOICE_TOKEN_CREATION_FAILED",
            ports::voice_service::VoiceServiceError::VoiceChannelCreation(_) => "VOICE_CHANNEL_CREATION_FAILED",
            ports::voice_service::VoiceServiceError::VoiceChannelDeletion(_) => "VOICE_CHANNEL_DELETION_FAILED",
            ports::voice_service::VoiceServiceError::ParticipantRemoval(_) => "VOICE_PARTICIPANT_REMOVAL_FAILED",
        }
    }
}

impl HttpMapping for ports::hasher::HashError {
    fn to_http_status(&self) -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }

    fn to_user_message(&self) -> String {
        "Authentication error".to_string()
    }

    fn to_error_code(&self) -> &'static str {
        "INTERNAL_ERROR"
    }
}

impl HttpMapping for ports::mailer::MailerError {
    fn to_http_status(&self) -> StatusCode {
        StatusCode::BAD_GATEWAY
    }

    fn to_user_message(&self) -> String {
        "Could not send email. Please try again in a moment.".to_string()
    }

    fn to_error_code(&self) -> &'static str {
        "MAILER_UNAVAILABLE"
    }
}

impl HttpMapping for ports::file_storage::FileStorageError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            ports::file_storage::FileStorageError::NotFound => StatusCode::NOT_FOUND,
            ports::file_storage::FileStorageError::Internal(_) => StatusCode::BAD_GATEWAY,
        }
    }

    fn to_user_message(&self) -> String {
        match self {
            ports::file_storage::FileStorageError::NotFound => "File not found.".to_string(),
            ports::file_storage::FileStorageError::Internal(_) => {
                "File storage is temporarily unavailable.".to_string()
            }
        }
    }

    fn to_error_code(&self) -> &'static str {
        match self {
            ports::file_storage::FileStorageError::NotFound => "ATTACHMENT_NOT_FOUND",
            ports::file_storage::FileStorageError::Internal(_) => "STORAGE_UNAVAILABLE",
        }
    }
}

impl HttpMapping for GetUserProfileError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            GetUserProfileError::Repository(e) => e.to_http_status(),
            GetUserProfileError::NotFound => StatusCode::NOT_FOUND,
        }
    }
    fn to_user_message(&self) -> String {
        match self {
            GetUserProfileError::Repository(e) => e.to_user_message(),
            GetUserProfileError::NotFound => "User not found".to_string(),
        }
    }
    fn to_error_code(&self) -> &'static str {
        match self {
            GetUserProfileError::Repository(e) => e.to_error_code(),
            GetUserProfileError::NotFound => "USER_NOT_FOUND",
        }
    }
}

impl HttpMapping for UpdatePrivacyError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            UpdatePrivacyError::Repository(e) => e.to_http_status(),
            UpdatePrivacyError::InvalidPolicy => StatusCode::BAD_REQUEST,
        }
    }
    fn to_user_message(&self) -> String {
        match self {
            UpdatePrivacyError::Repository(e) => e.to_user_message(),
            UpdatePrivacyError::InvalidPolicy => "Invalid DM policy".to_string(),
        }
    }
    fn to_error_code(&self) -> &'static str {
        match self {
            UpdatePrivacyError::Repository(e) => e.to_error_code(),
            UpdatePrivacyError::InvalidPolicy => "INVALID_DM_POLICY",
        }
    }
}

impl HttpMapping for CreateReportError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            CreateReportError::UnknownReason => StatusCode::BAD_REQUEST,
            CreateReportError::TargetNotFound => StatusCode::NOT_FOUND,
            CreateReportError::ContextMismatch => StatusCode::BAD_REQUEST,
            CreateReportError::AlreadyReported => StatusCode::CONFLICT,
            CreateReportError::TooManyReports => StatusCode::TOO_MANY_REQUESTS,
            CreateReportError::Domain(ReportError::SelfReport) => StatusCode::BAD_REQUEST,
            CreateReportError::Domain(_) => StatusCode::BAD_REQUEST,
            CreateReportError::Repository(e) => e.to_http_status(),
        }
    }
    fn to_user_message(&self) -> String {
        match self {
            CreateReportError::UnknownReason => "Unknown report reason.".to_string(),
            CreateReportError::TargetNotFound => "Reported target not found.".to_string(),
            CreateReportError::ContextMismatch => {
                "The reported message does not belong to this report.".to_string()
            }
            CreateReportError::AlreadyReported => {
                "You have already reported this in the last 24 hours.".to_string()
            }
            CreateReportError::TooManyReports => {
                "Too many reports sent today; try again later.".to_string()
            }
            CreateReportError::Domain(e) => e.to_string(),
            CreateReportError::Repository(e) => e.to_user_message(),
        }
    }
    fn to_error_code(&self) -> &'static str {
        match self {
            CreateReportError::UnknownReason => "REPORT_REASON_UNKNOWN",
            CreateReportError::TargetNotFound => "REPORT_TARGET_NOT_FOUND",
            CreateReportError::ContextMismatch => "REPORT_CONTEXT_MISMATCH",
            CreateReportError::AlreadyReported => "REPORT_ALREADY_SENT",
            CreateReportError::TooManyReports => "REPORT_LIMIT_REACHED",
            CreateReportError::Domain(ReportError::SelfReport) => "REPORT_SELF",
            CreateReportError::Domain(ReportError::ReasonNotAllowed) => "REPORT_REASON_NOT_ALLOWED",
            CreateReportError::Domain(ReportError::DetailsRequired) => "REPORT_DETAILS_REQUIRED",
            CreateReportError::Domain(ReportError::DetailsTooLong) => "REPORT_DETAILS_TOO_LONG",
            CreateReportError::Repository(e) => e.to_error_code(),
        }
    }
}

impl HttpMapping for BlockUserError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            BlockUserError::SelfBlock => StatusCode::BAD_REQUEST,
            BlockUserError::UserNotFound => StatusCode::NOT_FOUND,
            BlockUserError::Repository(e) => e.to_http_status(),
            BlockUserError::Domain(_) => StatusCode::CONFLICT,
        }
    }
    fn to_user_message(&self) -> String {
        match self {
            BlockUserError::SelfBlock => "You cannot block yourself.".to_string(),
            BlockUserError::UserNotFound => "User not found".to_string(),
            BlockUserError::Repository(e) => e.to_user_message(),
            BlockUserError::Domain(e) => e.to_string(),
        }
    }
    fn to_error_code(&self) -> &'static str {
        match self {
            BlockUserError::SelfBlock => "SELF_BLOCK",
            BlockUserError::UserNotFound => "USER_NOT_FOUND",
            BlockUserError::Repository(e) => e.to_error_code(),
            BlockUserError::Domain(_) => "FRIENDSHIP_CONFLICT",
        }
    }
}

impl HttpMapping for UnblockUserError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            UnblockUserError::NotBlocked => StatusCode::NOT_FOUND,
            UnblockUserError::Repository(e) => e.to_http_status(),
        }
    }
    fn to_user_message(&self) -> String {
        match self {
            UnblockUserError::NotBlocked => "This user is not blocked.".to_string(),
            UnblockUserError::Repository(e) => e.to_user_message(),
        }
    }
    fn to_error_code(&self) -> &'static str {
        match self {
            UnblockUserError::NotBlocked => "USER_NOT_BLOCKED",
            UnblockUserError::Repository(e) => e.to_error_code(),
        }
    }
}
