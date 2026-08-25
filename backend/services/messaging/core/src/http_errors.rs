use shared::api_error::{StatusCode, ToHttpStatus};
use crate::usecases::attachment::get_attachment::GetAttachmentError;
use crate::usecases::attachment::list_message_attachments::ListMessageAttachmentsError;
use crate::usecases::call::log_call::LogCallError;
use crate::usecases::channel::abort_voice_connection::AbortVoiceConnectionError;
use crate::usecases::channel::create_channel::CreateChannelError;
use crate::usecases::channel::delete_channel::DeleteChannelError;
use crate::usecases::channel::delete_channel_message::DeleteChannelMessageError;
use crate::usecases::channel::edit_channel_message::EditChannelMessageError;
use crate::usecases::channel::join_text_channel::JoinTextChannelError;
use crate::usecases::channel::join_voice_channel::JoinVoiceChannelError;
use crate::usecases::channel::join_voice_connection::JoinVoiceConnectionError;
use crate::usecases::channel::leave_voice_connection::LeaveVoiceConnectionError;
use crate::usecases::channel::list_channel_messages::ListChannelMessagesError;
use crate::usecases::channel::list_channel_participants::ListChannelParticipantsError;
use crate::usecases::channel::mute_channel::MuteChannelError;
use crate::usecases::channel::search_channel_messages::SearchChannelMessagesError;
use crate::usecases::channel::send_channel_message::SendChannelMessageError;
use crate::usecases::channel::track_media_connection::TrackMediaConnectionError;
use crate::usecases::channel::unmute_channel::UnmuteChannelError;
use crate::usecases::channel::update_stream_viewer::UpdateStreamViewerError;
use crate::usecases::dm::delete_dm_message::DeleteDmMessageError;
use crate::usecases::dm::edit_dm_message::EditDmMessageError;
use crate::usecases::dm::list_dm_messages::ListDmMessagesError;
use crate::usecases::dm::list_dm_previews::ListDmPreviewsError;
use crate::usecases::dm::search_dm_messages::SearchDmMessagesError;
use crate::usecases::dm::send_dm_message::SendDmMessageError;
use crate::usecases::friendship::accept_friend_request::AcceptFriendRequestError;
use crate::usecases::friendship::list_friends::ListFriendsError;
use crate::usecases::friendship::remove_friendship::RemoveFriendshipError;
use crate::usecases::friendship::send_friend_request::SendFriendRequestError;
use crate::usecases::notification::list_notification_preferences::ListNotificationPreferencesError;
use crate::usecases::notification::set_notification_preference::SetNotificationPreferenceError;
use crate::usecases::read_state::get_channel_read_receipts::GetChannelReadReceiptsError;
use crate::usecases::role::RoleError;
use crate::usecases::read_state::get_dm_read_receipt::GetDmReadReceiptError;
use crate::usecases::read_state::get_unread_counts::GetUnreadCountsError;
use crate::usecases::read_state::mark_channel_read::MarkChannelReadError;
use crate::usecases::read_state::mark_dm_read::MarkDmReadError;
use crate::usecases::server::accept_server_invite::AcceptServerInviteError;
use crate::usecases::server::create_server::CreateServerError;
use crate::usecases::server::decline_server_invite::DeclineServerInviteError;
use crate::usecases::server::delete_server::DeleteServerError;
use crate::usecases::server::get_voice_presence::GetVoicePresenceError;
use crate::usecases::server::invite_codes::CreateInviteCodeError;
use crate::usecases::server::invite_codes::ListInviteCodesError;
use crate::usecases::server::invite_codes::RedeemInviteCodeError;
use crate::usecases::server::invite_codes::RevokeInviteCodeError;
use crate::usecases::server::kick_member::KickMemberError;
use crate::usecases::server::leave_server::LeaveServerError;
use crate::usecases::server::list_sent_server_invites::ListSentServerInvitesError;
use crate::usecases::server::list_server_channels::ListServerChannelsError;
use crate::usecases::server::list_server_members::ListServerMembersError;
use crate::usecases::server::list_servers_by_user::ListServersByUserError;
use crate::usecases::server::send_server_invite::SendServerInviteError;
use crate::usecases::server::update_server::UpdateServerError;

impl ToHttpStatus for CreateChannelError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            CreateChannelError::Repository(e) => e.to_http_status(),
            CreateChannelError::Hash(e) => e.to_http_status(),
            CreateChannelError::NotAllowed => StatusCode::FORBIDDEN,
            CreateChannelError::PasswordRequired => StatusCode::BAD_REQUEST,
            CreateChannelError::InvalidData => StatusCode::BAD_REQUEST,
            CreateChannelError::Domain(e) => e.to_http_status(),
            CreateChannelError::Outbox(e) => e.to_http_status(),
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
            CreateChannelError::Outbox(e) => e.to_user_message(),
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
            CreateChannelError::Outbox(e) => e.to_error_code(),
        }
    }
}

impl ToHttpStatus for DeleteChannelError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            DeleteChannelError::ChannelNotFound => StatusCode::NOT_FOUND,
            DeleteChannelError::NotOwner => StatusCode::FORBIDDEN,
            DeleteChannelError::Repository(e) => e.to_http_status(),
            DeleteChannelError::Domain(e) => e.to_http_status(),
            DeleteChannelError::Outbox(e) => e.to_http_status(),
            DeleteChannelError::VoiceService(_) => StatusCode::BAD_GATEWAY,
        }
    }

    fn to_user_message(&self) -> String {
        match self {
            DeleteChannelError::ChannelNotFound => "Channel not found".to_string(),
            DeleteChannelError::NotOwner => "Only the channel owner can delete it.".to_string(),
            DeleteChannelError::Repository(e) => e.to_user_message(),
            DeleteChannelError::Domain(e) => e.to_user_message(),
            DeleteChannelError::Outbox(e) => e.to_user_message(),
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
            DeleteChannelError::Outbox(e) => e.to_error_code(),
            DeleteChannelError::VoiceService(e) => e.to_error_code(),
        }
    }
}

impl ToHttpStatus for MuteChannelError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            MuteChannelError::ChannelNotFound | MuteChannelError::TargetUserNotFound => {
                StatusCode::NOT_FOUND
            }
            MuteChannelError::VoiceNotSupported => StatusCode::BAD_REQUEST,
            MuteChannelError::Repository(e) => e.to_http_status(),
            MuteChannelError::Domain(e) => e.to_http_status(),
            MuteChannelError::Outbox(e) => e.to_http_status(),
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
            MuteChannelError::Outbox(e) => e.to_user_message(),
        }
    }

    fn to_error_code(&self) -> &'static str {
        match self {
            MuteChannelError::ChannelNotFound => "CHANNEL_NOT_FOUND",
            MuteChannelError::TargetUserNotFound => "TARGET_USER_NOT_FOUND",
            MuteChannelError::VoiceNotSupported => "VOICE_NOT_SUPPORTED",
            MuteChannelError::Repository(e) => e.to_error_code(),
            MuteChannelError::Domain(e) => e.to_error_code(),
            MuteChannelError::Outbox(e) => e.to_error_code(),
        }
    }
}

impl ToHttpStatus for UnmuteChannelError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            UnmuteChannelError::ChannelNotFound | UnmuteChannelError::TargetUserNotFound => {
                StatusCode::NOT_FOUND
            }
            UnmuteChannelError::VoiceNotSupported => StatusCode::BAD_REQUEST,
            UnmuteChannelError::Repository(e) => e.to_http_status(),
            UnmuteChannelError::Domain(e) => e.to_http_status(),
            UnmuteChannelError::Outbox(e) => e.to_http_status(),
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
            UnmuteChannelError::Outbox(e) => e.to_user_message(),
        }
    }

    fn to_error_code(&self) -> &'static str {
        match self {
            UnmuteChannelError::ChannelNotFound => "CHANNEL_NOT_FOUND",
            UnmuteChannelError::TargetUserNotFound => "TARGET_USER_NOT_FOUND",
            UnmuteChannelError::VoiceNotSupported => "VOICE_NOT_SUPPORTED",
            UnmuteChannelError::Repository(e) => e.to_error_code(),
            UnmuteChannelError::Domain(e) => e.to_error_code(),
            UnmuteChannelError::Outbox(e) => e.to_error_code(),
        }
    }
}

impl ToHttpStatus for AbortVoiceConnectionError {
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

impl ToHttpStatus for LeaveVoiceConnectionError {
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

impl ToHttpStatus for JoinVoiceConnectionError {
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

impl ToHttpStatus for TrackMediaConnectionError {
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

impl ToHttpStatus for CreateServerError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            CreateServerError::Repository(e) => e.to_http_status(),
            CreateServerError::Outbox(e) => e.to_http_status(),
        }
    }

    fn to_user_message(&self) -> String {
        match self {
            CreateServerError::Repository(e) => e.to_user_message(),
            CreateServerError::Outbox(e) => e.to_user_message(),
        }
    }

    fn to_error_code(&self) -> &'static str {
        match self {
            CreateServerError::Repository(e) => e.to_error_code(),
            CreateServerError::Outbox(e) => e.to_error_code(),
        }
    }
}

impl ToHttpStatus for DeleteServerError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            DeleteServerError::ServerNotFound => StatusCode::NOT_FOUND,
            DeleteServerError::NotOwner => StatusCode::FORBIDDEN,
            DeleteServerError::Repository(e) => e.to_http_status(),
            DeleteServerError::Outbox(e) => e.to_http_status(),
            DeleteServerError::VoiceService(_) => StatusCode::BAD_GATEWAY,
        }
    }

    fn to_user_message(&self) -> String {
        match self {
            DeleteServerError::ServerNotFound => "Server not found".to_string(),
            DeleteServerError::NotOwner => "Only the server owner can delete it.".to_string(),
            DeleteServerError::Repository(e) => e.to_user_message(),
            DeleteServerError::Outbox(e) => e.to_user_message(),
            DeleteServerError::VoiceService(_) => {
                "Voice service is temporarily unavailable.".to_string()
            }
        }
    }

    fn to_error_code(&self) -> &'static str {
        match self {
            DeleteServerError::ServerNotFound => "SERVER_NOT_FOUND",
            DeleteServerError::NotOwner => "SERVER_NOT_OWNER",
            DeleteServerError::Repository(e) => e.to_error_code(),
            DeleteServerError::Outbox(e) => e.to_error_code(),
            DeleteServerError::VoiceService(e) => e.to_error_code(),
        }
    }
}

impl ToHttpStatus for UpdateServerError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            UpdateServerError::ServerNotFound => StatusCode::NOT_FOUND,
            UpdateServerError::NotOwner => StatusCode::FORBIDDEN,
            UpdateServerError::NameTaken => StatusCode::CONFLICT,
            UpdateServerError::Outbox(e) => e.to_http_status(),
            UpdateServerError::Repository(e) => e.to_http_status(),
        }
    }

    fn to_user_message(&self) -> String {
        match self {
            UpdateServerError::ServerNotFound => "Server not found".to_string(),
            UpdateServerError::NotOwner => "Only the server owner can edit it.".to_string(),
            UpdateServerError::NameTaken => "A server with this name already exists.".to_string(),
            UpdateServerError::Outbox(e) => e.to_user_message(),
            UpdateServerError::Repository(e) => e.to_user_message(),
        }
    }

    fn to_error_code(&self) -> &'static str {
        match self {
            UpdateServerError::ServerNotFound => "SERVER_NOT_FOUND",
            UpdateServerError::NotOwner => "SERVER_NOT_OWNER",
            UpdateServerError::NameTaken => "SERVER_NAME_TAKEN",
            UpdateServerError::Outbox(e) => e.to_error_code(),
            UpdateServerError::Repository(e) => e.to_error_code(),
        }
    }
}

impl ToHttpStatus for CreateInviteCodeError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            CreateInviteCodeError::NotMember => StatusCode::FORBIDDEN,
            CreateInviteCodeError::NotAllowed => StatusCode::FORBIDDEN,
            CreateInviteCodeError::Repository(e) => e.to_http_status(),
        }
    }
    fn to_user_message(&self) -> String {
        match self {
            CreateInviteCodeError::NotAllowed => {
                "You don't have permission to manage invites".to_string()
            }
            CreateInviteCodeError::NotMember => {
                "You are not a member of this server.".to_string()
            }
            CreateInviteCodeError::Repository(e) => e.to_user_message(),
        }
    }
    fn to_error_code(&self) -> &'static str {
        match self {
            CreateInviteCodeError::NotMember => "SERVER_NOT_MEMBER",
            CreateInviteCodeError::NotAllowed => "INVITE_NOT_ALLOWED",
            CreateInviteCodeError::Repository(e) => e.to_error_code(),
        }
    }
}

impl ToHttpStatus for ListInviteCodesError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            ListInviteCodesError::NotMember => StatusCode::FORBIDDEN,
            ListInviteCodesError::NotAllowed => StatusCode::FORBIDDEN,
            ListInviteCodesError::Repository(e) => e.to_http_status(),
        }
    }
    fn to_user_message(&self) -> String {
        match self {
            ListInviteCodesError::NotAllowed => {
                "You don't have permission to manage invites".to_string()
            }
            ListInviteCodesError::NotMember => {
                "You are not a member of this server.".to_string()
            }
            ListInviteCodesError::Repository(e) => e.to_user_message(),
        }
    }
    fn to_error_code(&self) -> &'static str {
        match self {
            ListInviteCodesError::NotMember => "SERVER_NOT_MEMBER",
            ListInviteCodesError::NotAllowed => "INVITE_NOT_ALLOWED",
            ListInviteCodesError::Repository(e) => e.to_error_code(),
        }
    }
}

impl ToHttpStatus for RevokeInviteCodeError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            RevokeInviteCodeError::NotFound => StatusCode::NOT_FOUND,
            RevokeInviteCodeError::Repository(e) => e.to_http_status(),
        }
    }
    fn to_user_message(&self) -> String {
        match self {
            RevokeInviteCodeError::NotFound => "Invite not found.".to_string(),
            RevokeInviteCodeError::Repository(e) => e.to_user_message(),
        }
    }
    fn to_error_code(&self) -> &'static str {
        match self {
            RevokeInviteCodeError::NotFound => "INVITE_NOT_FOUND",
            RevokeInviteCodeError::Repository(e) => e.to_error_code(),
        }
    }
}

impl ToHttpStatus for RedeemInviteCodeError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            RedeemInviteCodeError::InvalidCode => StatusCode::NOT_FOUND,
            RedeemInviteCodeError::ServerNotFound => StatusCode::NOT_FOUND,
            RedeemInviteCodeError::Repository(e) => e.to_http_status(),
        }
    }
    fn to_user_message(&self) -> String {
        match self {
            RedeemInviteCodeError::InvalidCode => {
                "This invite code is invalid or has expired.".to_string()
            }
            RedeemInviteCodeError::ServerNotFound => "Server not found".to_string(),
            RedeemInviteCodeError::Repository(e) => e.to_user_message(),
        }
    }
    fn to_error_code(&self) -> &'static str {
        match self {
            RedeemInviteCodeError::InvalidCode => "INVITE_CODE_INVALID",
            RedeemInviteCodeError::ServerNotFound => "SERVER_NOT_FOUND",
            RedeemInviteCodeError::Repository(e) => e.to_error_code(),
        }
    }
}

impl ToHttpStatus for JoinVoiceChannelError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            JoinVoiceChannelError::ChannelNotFound | JoinVoiceChannelError::UserNotFound => {
                StatusCode::NOT_FOUND
            }
            JoinVoiceChannelError::PasswordRequired => StatusCode::BAD_REQUEST,
            JoinVoiceChannelError::InvalidPassword => StatusCode::FORBIDDEN,
            JoinVoiceChannelError::VoiceNotSupported => StatusCode::BAD_REQUEST,
            JoinVoiceChannelError::NotMember => StatusCode::FORBIDDEN,
            JoinVoiceChannelError::Repository(e) => e.to_http_status(),
            JoinVoiceChannelError::VoiceService(e) => e.to_http_status(),
            JoinVoiceChannelError::Hash(e) => e.to_http_status(),
            JoinVoiceChannelError::Domain(e) => e.to_http_status(),
            JoinVoiceChannelError::Outbox(e) => e.to_http_status(),
        }
    }

    fn to_user_message(&self) -> String {
        match self {
            JoinVoiceChannelError::ChannelNotFound => "Channel not found".to_string(),
            JoinVoiceChannelError::UserNotFound => "User not found".to_string(),
            JoinVoiceChannelError::PasswordRequired => {
                "Password is required to join this channel".to_string()
            }
            JoinVoiceChannelError::InvalidPassword => "Invalid password".to_string(),
            JoinVoiceChannelError::VoiceNotSupported => {
                "Voice is not supported for this channel".to_string()
            }
            JoinVoiceChannelError::NotMember => "You are not a member of this server".to_string(),
            JoinVoiceChannelError::Repository(e) => e.to_user_message(),
            JoinVoiceChannelError::VoiceService(e) => e.to_user_message(),
            JoinVoiceChannelError::Hash(e) => e.to_user_message(),
            JoinVoiceChannelError::Domain(e) => e.to_user_message(),
            JoinVoiceChannelError::Outbox(e) => e.to_user_message(),
        }
    }

    fn to_error_code(&self) -> &'static str {
        match self {
            JoinVoiceChannelError::ChannelNotFound => "CHANNEL_NOT_FOUND",
            JoinVoiceChannelError::UserNotFound => "USER_NOT_FOUND",
            JoinVoiceChannelError::PasswordRequired => "CHANNEL_PASSWORD_REQUIRED",
            JoinVoiceChannelError::InvalidPassword => "INVALID_CHANNEL_PASSWORD",
            JoinVoiceChannelError::VoiceNotSupported => "VOICE_NOT_SUPPORTED",
            JoinVoiceChannelError::NotMember => "SERVER_NOT_MEMBER",
            JoinVoiceChannelError::Repository(e) => e.to_error_code(),
            JoinVoiceChannelError::VoiceService(e) => e.to_error_code(),
            JoinVoiceChannelError::Hash(e) => e.to_error_code(),
            JoinVoiceChannelError::Domain(e) => e.to_error_code(),
            JoinVoiceChannelError::Outbox(e) => e.to_error_code(),
        }
    }
}

impl ToHttpStatus for LogCallError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            LogCallError::Repository(e) => e.to_http_status(),
            LogCallError::Domain(e) => e.to_http_status(),
            LogCallError::Outbox(_) => StatusCode::INTERNAL_SERVER_ERROR,
            LogCallError::NotFriends => StatusCode::FORBIDDEN,
        }
    }
    fn to_user_message(&self) -> String {
        match self {
            LogCallError::Repository(e) => e.to_user_message(),
            LogCallError::Domain(e) => e.to_user_message(),
            LogCallError::Outbox(_) => "Failed to log call".to_string(),
            LogCallError::NotFriends => "You can only call your friends".to_string(),
        }
    }
    fn to_error_code(&self) -> &'static str {
        match self {
            LogCallError::Repository(e) => e.to_error_code(),
            LogCallError::Domain(e) => e.to_error_code(),
            LogCallError::Outbox(_) => "CALL_LOG_FAILED",
            LogCallError::NotFriends => "CALL_NOT_FRIENDS",
        }
    }
}

shared::impl_to_http_status_delegate!(ListServersByUserError, Repository);

impl ToHttpStatus for ListServerMembersError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            ListServerMembersError::Repository(e) => e.to_http_status(),
            ListServerMembersError::NotMember => StatusCode::FORBIDDEN,
        }
    }

    fn to_user_message(&self) -> String {
        match self {
            ListServerMembersError::Repository(e) => e.to_user_message(),
            ListServerMembersError::NotMember => {
                "You are not a member of this server".to_string()
            }
        }
    }

    fn to_error_code(&self) -> &'static str {
        match self {
            ListServerMembersError::Repository(e) => e.to_error_code(),
            ListServerMembersError::NotMember => "SERVER_NOT_MEMBER",
        }
    }
}

impl ToHttpStatus for GetVoicePresenceError {
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
                "You are not a member of this server".to_string()
            }
        }
    }

    fn to_error_code(&self) -> &'static str {
        match self {
            GetVoicePresenceError::Repository(e) => e.to_error_code(),
            GetVoicePresenceError::NotMember => "SERVER_NOT_MEMBER",
        }
    }
}

impl ToHttpStatus for UpdateStreamViewerError {
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
                "You are not a member of this server".to_string()
            }
        }
    }

    fn to_error_code(&self) -> &'static str {
        match self {
            UpdateStreamViewerError::Repository(e) => e.to_error_code(),
            UpdateStreamViewerError::NotMember => "SERVER_NOT_MEMBER",
        }
    }
}

impl ToHttpStatus for ListServerChannelsError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            ListServerChannelsError::Repository(e) => e.to_http_status(),
            ListServerChannelsError::NotMember => StatusCode::FORBIDDEN,
        }
    }

    fn to_user_message(&self) -> String {
        match self {
            ListServerChannelsError::Repository(e) => e.to_user_message(),
            ListServerChannelsError::NotMember => {
                "You are not a member of this server".to_string()
            }
        }
    }

    fn to_error_code(&self) -> &'static str {
        match self {
            ListServerChannelsError::Repository(e) => e.to_error_code(),
            ListServerChannelsError::NotMember => "SERVER_NOT_MEMBER",
        }
    }
}

impl ToHttpStatus for JoinTextChannelError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            JoinTextChannelError::Repository(e) => e.to_http_status(),
            JoinTextChannelError::Domain(e) => e.to_http_status(),
            JoinTextChannelError::Outbox(e) => e.to_http_status(),
            JoinTextChannelError::Hash(e) => e.to_http_status(),
            JoinTextChannelError::NotServerMember => StatusCode::FORBIDDEN,
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
            JoinTextChannelError::Outbox(e) => e.to_user_message(),
            JoinTextChannelError::Hash(e) => e.to_user_message(),
            JoinTextChannelError::NotServerMember => {
                "You are not a member of this server".to_string()
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
            JoinTextChannelError::Outbox(e) => e.to_error_code(),
            JoinTextChannelError::Hash(e) => e.to_error_code(),
            JoinTextChannelError::NotServerMember => "SERVER_NOT_MEMBER",
            JoinTextChannelError::ChannelNotFound => "CHANNEL_NOT_FOUND",
            JoinTextChannelError::NotTextChannel => "CHANNEL_NOT_TEXT",
            JoinTextChannelError::PasswordRequired => "CHANNEL_PASSWORD_REQUIRED",
            JoinTextChannelError::InvalidPassword => "INVALID_CHANNEL_PASSWORD",
        }
    }
}

impl ToHttpStatus for SendChannelMessageError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            SendChannelMessageError::Repository(e) => e.to_http_status(),
            SendChannelMessageError::Domain(e) => e.to_http_status(),
            SendChannelMessageError::Channel(e) => e.to_http_status(),
            SendChannelMessageError::Outbox(e) => e.to_http_status(),
            SendChannelMessageError::NotServerMember
            | SendChannelMessageError::NotChannelMember => StatusCode::FORBIDDEN,
            SendChannelMessageError::ChannelNotFound => StatusCode::NOT_FOUND,
            SendChannelMessageError::NotTextChannel => StatusCode::BAD_REQUEST,
            SendChannelMessageError::InvalidAttachments => StatusCode::BAD_REQUEST,
            SendChannelMessageError::AttachmentsUnavailable(_) => StatusCode::SERVICE_UNAVAILABLE,
        }
    }

    fn to_user_message(&self) -> String {
        match self {
            SendChannelMessageError::Repository(e) => e.to_user_message(),
            SendChannelMessageError::Domain(e) => e.to_user_message(),
            SendChannelMessageError::Channel(e) => e.to_user_message(),
            SendChannelMessageError::Outbox(e) => e.to_user_message(),
            SendChannelMessageError::NotServerMember => {
                "You are not a member of this server".to_string()
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
        }
    }

    fn to_error_code(&self) -> &'static str {
        match self {
            SendChannelMessageError::Repository(e) => e.to_error_code(),
            SendChannelMessageError::Domain(e) => e.to_error_code(),
            SendChannelMessageError::Channel(e) => e.to_error_code(),
            SendChannelMessageError::Outbox(e) => e.to_error_code(),
            SendChannelMessageError::NotServerMember => "SERVER_NOT_MEMBER",
            SendChannelMessageError::NotChannelMember => "CHANNEL_NOT_MEMBER",
            SendChannelMessageError::ChannelNotFound => "CHANNEL_NOT_FOUND",
            SendChannelMessageError::InvalidAttachments => "ATTACHMENTS_INVALID",
            SendChannelMessageError::AttachmentsUnavailable(_) => "ATTACHMENTS_UNAVAILABLE",
            SendChannelMessageError::NotTextChannel => "CHANNEL_NOT_TEXT",
        }
    }
}

impl ToHttpStatus for EditChannelMessageError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            EditChannelMessageError::Repository(e) => e.to_http_status(),
            EditChannelMessageError::Domain(e) => e.to_http_status(),
            EditChannelMessageError::Outbox(e) => e.to_http_status(),
            EditChannelMessageError::NotServerMember => StatusCode::FORBIDDEN,
            EditChannelMessageError::MessageNotFound => StatusCode::NOT_FOUND,
            EditChannelMessageError::InvalidAttachments => StatusCode::BAD_REQUEST,
            EditChannelMessageError::AttachmentsUnavailable(_) => StatusCode::SERVICE_UNAVAILABLE,
        }
    }

    fn to_user_message(&self) -> String {
        match self {
            EditChannelMessageError::Repository(e) => e.to_user_message(),
            EditChannelMessageError::Domain(e) => e.to_user_message(),
            EditChannelMessageError::Outbox(e) => e.to_user_message(),
            EditChannelMessageError::NotServerMember => {
                "You are not a member of this server".to_string()
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
            EditChannelMessageError::Outbox(e) => e.to_error_code(),
            EditChannelMessageError::NotServerMember => "SERVER_NOT_MEMBER",
            EditChannelMessageError::MessageNotFound => "MESSAGE_NOT_FOUND",
            EditChannelMessageError::InvalidAttachments => "ATTACHMENTS_INVALID",
            EditChannelMessageError::AttachmentsUnavailable(_) => "ATTACHMENTS_UNAVAILABLE",
        }
    }
}

impl ToHttpStatus for DeleteChannelMessageError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            DeleteChannelMessageError::Repository(e) => e.to_http_status(),
            DeleteChannelMessageError::Domain(e) => e.to_http_status(),
            DeleteChannelMessageError::Outbox(e) => e.to_http_status(),
            DeleteChannelMessageError::NotServerMember => StatusCode::FORBIDDEN,
            DeleteChannelMessageError::MessageNotFound => StatusCode::NOT_FOUND,
        }
    }

    fn to_user_message(&self) -> String {
        match self {
            DeleteChannelMessageError::Repository(e) => e.to_user_message(),
            DeleteChannelMessageError::Domain(e) => e.to_user_message(),
            DeleteChannelMessageError::Outbox(e) => e.to_user_message(),
            DeleteChannelMessageError::NotServerMember => {
                "You are not a member of this server".to_string()
            }
            DeleteChannelMessageError::MessageNotFound => "Message not found".to_string(),
        }
    }

    fn to_error_code(&self) -> &'static str {
        match self {
            DeleteChannelMessageError::Repository(e) => e.to_error_code(),
            DeleteChannelMessageError::Domain(e) => e.to_error_code(),
            DeleteChannelMessageError::Outbox(e) => e.to_error_code(),
            DeleteChannelMessageError::NotServerMember => "SERVER_NOT_MEMBER",
            DeleteChannelMessageError::MessageNotFound => "MESSAGE_NOT_FOUND",
        }
    }
}

impl ToHttpStatus for ListChannelMessagesError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            ListChannelMessagesError::Repository(e) => e.to_http_status(),
            ListChannelMessagesError::NotServerMember => StatusCode::FORBIDDEN,
            ListChannelMessagesError::NotChannelMember => StatusCode::FORBIDDEN,
            ListChannelMessagesError::ChannelNotFound => StatusCode::NOT_FOUND,
            ListChannelMessagesError::NotTextChannel => StatusCode::BAD_REQUEST,
            ListChannelMessagesError::InvalidCursor => StatusCode::BAD_REQUEST,
            ListChannelMessagesError::ConflictingCursors => StatusCode::BAD_REQUEST,
        }
    }

    fn to_user_message(&self) -> String {
        match self {
            ListChannelMessagesError::Repository(e) => e.to_user_message(),
            ListChannelMessagesError::NotServerMember => {
                "You are not a member of this server".to_string()
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
        }
    }

    fn to_error_code(&self) -> &'static str {
        match self {
            ListChannelMessagesError::Repository(e) => e.to_error_code(),
            ListChannelMessagesError::NotServerMember => "SERVER_NOT_MEMBER",
            ListChannelMessagesError::NotChannelMember => "CHANNEL_NOT_MEMBER",
            ListChannelMessagesError::ChannelNotFound => "CHANNEL_NOT_FOUND",
            ListChannelMessagesError::NotTextChannel => "CHANNEL_NOT_TEXT",
            ListChannelMessagesError::InvalidCursor => "INVALID_CURSOR",
            ListChannelMessagesError::ConflictingCursors => "CONFLICTING_CURSORS",
        }
    }
}

impl ToHttpStatus for SearchChannelMessagesError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            SearchChannelMessagesError::Repository(e) => e.to_http_status(),
            SearchChannelMessagesError::NotServerMember => StatusCode::FORBIDDEN,
            SearchChannelMessagesError::NotChannelMember => StatusCode::FORBIDDEN,
            SearchChannelMessagesError::ChannelNotFound => StatusCode::NOT_FOUND,
            SearchChannelMessagesError::NotTextChannel => StatusCode::BAD_REQUEST,
            SearchChannelMessagesError::EmptyQuery => StatusCode::BAD_REQUEST,
        }
    }

    fn to_user_message(&self) -> String {
        match self {
            SearchChannelMessagesError::Repository(e) => e.to_user_message(),
            SearchChannelMessagesError::NotServerMember => {
                "You are not a member of this server".to_string()
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
            SearchChannelMessagesError::NotServerMember => "SERVER_NOT_MEMBER",
            SearchChannelMessagesError::NotChannelMember => "CHANNEL_NOT_MEMBER",
            SearchChannelMessagesError::ChannelNotFound => "CHANNEL_NOT_FOUND",
            SearchChannelMessagesError::NotTextChannel => "CHANNEL_NOT_TEXT",
            SearchChannelMessagesError::EmptyQuery => "EMPTY_QUERY",
        }
    }
}

impl ToHttpStatus for SendFriendRequestError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            SendFriendRequestError::Repository(e) => e.to_http_status(),
            SendFriendRequestError::Domain(e) => e.to_http_status(),
            SendFriendRequestError::Outbox(e) => e.to_http_status(),
            SendFriendRequestError::InvalidUsername => StatusCode::BAD_REQUEST,
            SendFriendRequestError::UserNotFound => StatusCode::NOT_FOUND,
            SendFriendRequestError::SelfTarget => StatusCode::BAD_REQUEST,
            SendFriendRequestError::AlreadyRequested => StatusCode::CONFLICT,
            SendFriendRequestError::AlreadyFriends => StatusCode::CONFLICT,
            SendFriendRequestError::IncomingPending => StatusCode::CONFLICT,
        }
    }

    fn to_user_message(&self) -> String {
        match self {
            SendFriendRequestError::Repository(e) => e.to_user_message(),
            SendFriendRequestError::Domain(e) => e.to_user_message(),
            SendFriendRequestError::Outbox(e) => e.to_user_message(),
            SendFriendRequestError::InvalidUsername => "Invalid username".to_string(),
            SendFriendRequestError::UserNotFound => "User not found".to_string(),
            SendFriendRequestError::SelfTarget => {
                "You cannot send a friend request to yourself".to_string()
            }
            SendFriendRequestError::AlreadyRequested => {
                "Friend request already sent".to_string()
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
            SendFriendRequestError::Outbox(e) => e.to_error_code(),
            SendFriendRequestError::InvalidUsername => "INVALID_USERNAME",
            SendFriendRequestError::UserNotFound => "USER_NOT_FOUND",
            SendFriendRequestError::SelfTarget => "SELF_FRIEND_REQUEST",
            SendFriendRequestError::AlreadyRequested => "FRIEND_REQUEST_ALREADY_SENT",
            SendFriendRequestError::AlreadyFriends => "ALREADY_FRIENDS",
            SendFriendRequestError::IncomingPending => "INCOMING_REQUEST_PENDING",
        }
    }
}

impl ToHttpStatus for AcceptFriendRequestError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            AcceptFriendRequestError::Repository(e) => e.to_http_status(),
            AcceptFriendRequestError::Domain(e) => e.to_http_status(),
            AcceptFriendRequestError::Outbox(e) => e.to_http_status(),
            AcceptFriendRequestError::NoPendingRequest => StatusCode::NOT_FOUND,
        }
    }

    fn to_user_message(&self) -> String {
        match self {
            AcceptFriendRequestError::Repository(e) => e.to_user_message(),
            AcceptFriendRequestError::Domain(e) => e.to_user_message(),
            AcceptFriendRequestError::Outbox(e) => e.to_user_message(),
            AcceptFriendRequestError::NoPendingRequest => {
                "No pending friend request from this user".to_string()
            }
        }
    }

    fn to_error_code(&self) -> &'static str {
        match self {
            AcceptFriendRequestError::Repository(e) => e.to_error_code(),
            AcceptFriendRequestError::Domain(e) => e.to_error_code(),
            AcceptFriendRequestError::Outbox(e) => e.to_error_code(),
            AcceptFriendRequestError::NoPendingRequest => "NO_PENDING_REQUEST",
        }
    }
}

impl ToHttpStatus for RemoveFriendshipError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            RemoveFriendshipError::Repository(e) => e.to_http_status(),
            RemoveFriendshipError::Outbox(e) => e.to_http_status(),
            RemoveFriendshipError::NotFound => StatusCode::NOT_FOUND,
        }
    }

    fn to_user_message(&self) -> String {
        match self {
            RemoveFriendshipError::Repository(e) => e.to_user_message(),
            RemoveFriendshipError::Outbox(e) => e.to_user_message(),
            RemoveFriendshipError::NotFound => "Friendship not found".to_string(),
        }
    }

    fn to_error_code(&self) -> &'static str {
        match self {
            RemoveFriendshipError::Repository(e) => e.to_error_code(),
            RemoveFriendshipError::Outbox(e) => e.to_error_code(),
            RemoveFriendshipError::NotFound => "FRIENDSHIP_NOT_FOUND",
        }
    }
}

impl ToHttpStatus for SendDmMessageError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            SendDmMessageError::Repository(e) => e.to_http_status(),
            SendDmMessageError::Domain(e) => e.to_http_status(),
            SendDmMessageError::Outbox(e) => e.to_http_status(),
            SendDmMessageError::NotFriends => StatusCode::FORBIDDEN,
            SendDmMessageError::InvalidAttachments => StatusCode::BAD_REQUEST,
            SendDmMessageError::AttachmentsUnavailable(_) => StatusCode::SERVICE_UNAVAILABLE,
        }
    }
    fn to_user_message(&self) -> String {
        match self {
            SendDmMessageError::Repository(e) => e.to_user_message(),
            SendDmMessageError::Domain(e) => e.to_user_message(),
            SendDmMessageError::Outbox(e) => e.to_user_message(),
            SendDmMessageError::InvalidAttachments => {
                "One or more attachments are invalid.".to_string()
            }
            SendDmMessageError::AttachmentsUnavailable(_) => {
                "Attachment service is temporarily unavailable. Try again later.".to_string()
            }
            SendDmMessageError::NotFriends => {
                "You can only DM users who are your friends".to_string()
            }
        }
    }
    fn to_error_code(&self) -> &'static str {
        match self {
            SendDmMessageError::Repository(e) => e.to_error_code(),
            SendDmMessageError::Domain(e) => e.to_error_code(),
            SendDmMessageError::Outbox(e) => e.to_error_code(),
            SendDmMessageError::InvalidAttachments => "ATTACHMENTS_INVALID",
            SendDmMessageError::AttachmentsUnavailable(_) => "ATTACHMENTS_UNAVAILABLE",
            SendDmMessageError::NotFriends => "NOT_FRIENDS",
        }
    }
}

impl ToHttpStatus for SendServerInviteError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            SendServerInviteError::Repository(e) => e.to_http_status(),
            SendServerInviteError::Domain(e) => e.to_http_status(),
            SendServerInviteError::Outbox(e) => e.to_http_status(),
            SendServerInviteError::NotFriends => StatusCode::FORBIDDEN,
            SendServerInviteError::ServerNotFound => StatusCode::NOT_FOUND,
            SendServerInviteError::NotServerMember => StatusCode::FORBIDDEN,
            SendServerInviteError::NotAllowed => StatusCode::FORBIDDEN,
            SendServerInviteError::AlreadyMember => StatusCode::CONFLICT,
            SendServerInviteError::InviteAlreadyPending => StatusCode::CONFLICT,
        }
    }
    fn to_user_message(&self) -> String {
        match self {
            SendServerInviteError::Repository(e) => e.to_user_message(),
            SendServerInviteError::Domain(e) => e.to_user_message(),
            SendServerInviteError::Outbox(e) => e.to_user_message(),
            SendServerInviteError::NotFriends => {
                "You can only invite users who are your friends".to_string()
            }
            SendServerInviteError::ServerNotFound => "Server not found".to_string(),
            SendServerInviteError::NotServerMember => {
                "Only members of this server can invite people".to_string()
            }
            SendServerInviteError::NotAllowed => {
                "You don't have permission to manage invites".to_string()
            }
            SendServerInviteError::AlreadyMember => {
                "This user is already a member of the server".to_string()
            }
            SendServerInviteError::InviteAlreadyPending => {
                "You've already invited this user to this server".to_string()
            }
        }
    }
    fn to_error_code(&self) -> &'static str {
        match self {
            SendServerInviteError::Repository(e) => e.to_error_code(),
            SendServerInviteError::Domain(e) => e.to_error_code(),
            SendServerInviteError::Outbox(e) => e.to_error_code(),
            SendServerInviteError::NotFriends => "NOT_FRIENDS",
            SendServerInviteError::ServerNotFound => "SERVER_NOT_FOUND",
            SendServerInviteError::NotServerMember => "SERVER_NOT_MEMBER",
            SendServerInviteError::NotAllowed => "INVITE_NOT_ALLOWED",
            SendServerInviteError::AlreadyMember => "ALREADY_MEMBER",
            SendServerInviteError::InviteAlreadyPending => "INVITE_ALREADY_PENDING",
        }
    }
}

impl ToHttpStatus for KickMemberError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            KickMemberError::Outbox(e) => e.to_http_status(),
            KickMemberError::Repository(e) => e.to_http_status(),
            KickMemberError::NotMember => StatusCode::NOT_FOUND,
            KickMemberError::NotAllowed => StatusCode::FORBIDDEN,
        }
    }

    fn to_user_message(&self) -> String {
        match self {
            KickMemberError::Outbox(e) => e.to_user_message(),
            KickMemberError::Repository(e) => e.to_user_message(),
            KickMemberError::NotMember => "User is not a member of this server".to_string(),
            KickMemberError::NotAllowed => {
                "You don't have permission to kick this member".to_string()
            }
        }
    }

    fn to_error_code(&self) -> &'static str {
        match self {
            KickMemberError::Outbox(e) => e.to_error_code(),
            KickMemberError::Repository(e) => e.to_error_code(),
            KickMemberError::NotMember => "SERVER_NOT_MEMBER",
            KickMemberError::NotAllowed => "KICK_NOT_ALLOWED",
        }
    }
}

impl ToHttpStatus for LeaveServerError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            LeaveServerError::Repository(e) => e.to_http_status(),
            LeaveServerError::Outbox(e) => e.to_http_status(),
            LeaveServerError::NotMember => StatusCode::NOT_FOUND,
            LeaveServerError::OwnerCannotLeave => StatusCode::CONFLICT,
        }
    }
    fn to_user_message(&self) -> String {
        match self {
            LeaveServerError::Repository(e) => e.to_user_message(),
            LeaveServerError::Outbox(e) => e.to_user_message(),
            LeaveServerError::NotMember => {
                "You are not a member of this server".to_string()
            }
            LeaveServerError::OwnerCannotLeave => {
                "Server owners cannot leave their own server; delete it instead".to_string()
            }
        }
    }
    fn to_error_code(&self) -> &'static str {
        match self {
            LeaveServerError::Repository(e) => e.to_error_code(),
            LeaveServerError::Outbox(e) => e.to_error_code(),
            LeaveServerError::NotMember => "SERVER_NOT_MEMBER",
            LeaveServerError::OwnerCannotLeave => "SERVER_OWNER_CANNOT_LEAVE",
        }
    }
}

impl ToHttpStatus for ListSentServerInvitesError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            ListSentServerInvitesError::Repository(e) => e.to_http_status(),
            ListSentServerInvitesError::NotMember => StatusCode::FORBIDDEN,
        }
    }
    fn to_user_message(&self) -> String {
        match self {
            ListSentServerInvitesError::Repository(e) => e.to_user_message(),
            ListSentServerInvitesError::NotMember => {
                "You are not a member of this server".to_string()
            }
        }
    }
    fn to_error_code(&self) -> &'static str {
        match self {
            ListSentServerInvitesError::Repository(e) => e.to_error_code(),
            ListSentServerInvitesError::NotMember => "SERVER_NOT_MEMBER",
        }
    }
}

impl ToHttpStatus for DeclineServerInviteError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            DeclineServerInviteError::Repository(e) => e.to_http_status(),
            DeclineServerInviteError::Outbox(e) => e.to_http_status(),
            DeclineServerInviteError::NotAnInvite => StatusCode::BAD_REQUEST,
            DeclineServerInviteError::NotRecipient => StatusCode::FORBIDDEN,
        }
    }
    fn to_user_message(&self) -> String {
        match self {
            DeclineServerInviteError::Repository(e) => e.to_user_message(),
            DeclineServerInviteError::Outbox(e) => e.to_user_message(),
            DeclineServerInviteError::NotAnInvite => {
                "This message is not a server invite".to_string()
            }
            DeclineServerInviteError::NotRecipient => {
                "This invite was not addressed to you".to_string()
            }
        }
    }
    fn to_error_code(&self) -> &'static str {
        match self {
            DeclineServerInviteError::Repository(e) => e.to_error_code(),
            DeclineServerInviteError::Outbox(e) => e.to_error_code(),
            DeclineServerInviteError::NotAnInvite => "INVITE_INVALID",
            DeclineServerInviteError::NotRecipient => "INVITE_NOT_RECIPIENT",
        }
    }
}

impl ToHttpStatus for AcceptServerInviteError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            AcceptServerInviteError::Repository(e) => e.to_http_status(),
            AcceptServerInviteError::Outbox(e) => e.to_http_status(),
            AcceptServerInviteError::NotAnInvite => StatusCode::BAD_REQUEST,
            AcceptServerInviteError::NotRecipient => StatusCode::FORBIDDEN,
        }
    }
    fn to_user_message(&self) -> String {
        match self {
            AcceptServerInviteError::Repository(e) => e.to_user_message(),
            AcceptServerInviteError::Outbox(e) => e.to_user_message(),
            AcceptServerInviteError::NotAnInvite => {
                "This message is not a server invite".to_string()
            }
            AcceptServerInviteError::NotRecipient => {
                "This invite was not addressed to you".to_string()
            }
        }
    }
    fn to_error_code(&self) -> &'static str {
        match self {
            AcceptServerInviteError::Repository(e) => e.to_error_code(),
            AcceptServerInviteError::Outbox(e) => e.to_error_code(),
            AcceptServerInviteError::NotAnInvite => "INVITE_INVALID",
            AcceptServerInviteError::NotRecipient => "INVITE_NOT_RECIPIENT",
        }
    }
}

impl ToHttpStatus for EditDmMessageError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            EditDmMessageError::Repository(e) => e.to_http_status(),
            EditDmMessageError::Dm(e) => e.to_http_status(),
            EditDmMessageError::Message(e) => e.to_http_status(),
            EditDmMessageError::Outbox(e) => e.to_http_status(),
            EditDmMessageError::NotFound => StatusCode::NOT_FOUND,
            EditDmMessageError::InvalidAttachments => StatusCode::BAD_REQUEST,
            EditDmMessageError::AttachmentsUnavailable(_) => StatusCode::SERVICE_UNAVAILABLE,
        }
    }
    fn to_user_message(&self) -> String {
        match self {
            EditDmMessageError::Repository(e) => e.to_user_message(),
            EditDmMessageError::Dm(e) => e.to_user_message(),
            EditDmMessageError::Message(e) => e.to_user_message(),
            EditDmMessageError::Outbox(e) => e.to_user_message(),
            EditDmMessageError::NotFound => "Message not found".to_string(),
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
            EditDmMessageError::Outbox(e) => e.to_error_code(),
            EditDmMessageError::NotFound => "DM_NOT_FOUND",
            EditDmMessageError::InvalidAttachments => "ATTACHMENTS_INVALID",
            EditDmMessageError::AttachmentsUnavailable(_) => "ATTACHMENTS_UNAVAILABLE",
        }
    }
}

impl ToHttpStatus for DeleteDmMessageError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            DeleteDmMessageError::Repository(e) => e.to_http_status(),
            DeleteDmMessageError::Dm(e) => e.to_http_status(),
            DeleteDmMessageError::Outbox(e) => e.to_http_status(),
            DeleteDmMessageError::NotFound => StatusCode::NOT_FOUND,
        }
    }
    fn to_user_message(&self) -> String {
        match self {
            DeleteDmMessageError::Repository(e) => e.to_user_message(),
            DeleteDmMessageError::Dm(e) => e.to_user_message(),
            DeleteDmMessageError::Outbox(e) => e.to_user_message(),
            DeleteDmMessageError::NotFound => "Message not found".to_string(),
        }
    }
    fn to_error_code(&self) -> &'static str {
        match self {
            DeleteDmMessageError::Repository(e) => e.to_error_code(),
            DeleteDmMessageError::Dm(e) => e.to_error_code(),
            DeleteDmMessageError::Outbox(e) => e.to_error_code(),
            DeleteDmMessageError::NotFound => "DM_NOT_FOUND",
        }
    }
}

impl ToHttpStatus for GetAttachmentError {
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

impl ToHttpStatus for ListMessageAttachmentsError {
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

impl ToHttpStatus for ListDmPreviewsError {
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

impl ToHttpStatus for ListDmMessagesError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            ListDmMessagesError::Repository(e) => e.to_http_status(),
            ListDmMessagesError::NotFriends => StatusCode::FORBIDDEN,
            ListDmMessagesError::InvalidCursor
            | ListDmMessagesError::ConflictingCursors => StatusCode::BAD_REQUEST,
        }
    }
    fn to_user_message(&self) -> String {
        match self {
            ListDmMessagesError::Repository(e) => e.to_user_message(),
            ListDmMessagesError::NotFriends => {
                "You can only view DMs with friends".to_string()
            }
            ListDmMessagesError::InvalidCursor => "Invalid pagination cursor".to_string(),
            ListDmMessagesError::ConflictingCursors => {
                "Cannot specify both before and after cursors".to_string()
            }
        }
    }
    fn to_error_code(&self) -> &'static str {
        match self {
            ListDmMessagesError::Repository(e) => e.to_error_code(),
            ListDmMessagesError::NotFriends => "NOT_FRIENDS",
            ListDmMessagesError::InvalidCursor => "INVALID_CURSOR",
            ListDmMessagesError::ConflictingCursors => "CONFLICTING_CURSORS",
        }
    }
}

impl ToHttpStatus for SearchDmMessagesError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            SearchDmMessagesError::Repository(e) => e.to_http_status(),
            SearchDmMessagesError::NotFriends => StatusCode::FORBIDDEN,
            SearchDmMessagesError::EmptyQuery => StatusCode::BAD_REQUEST,
        }
    }
    fn to_user_message(&self) -> String {
        match self {
            SearchDmMessagesError::Repository(e) => e.to_user_message(),
            SearchDmMessagesError::NotFriends => {
                "You can only search DMs with friends".to_string()
            }
            SearchDmMessagesError::EmptyQuery => "Search query cannot be empty".to_string(),
        }
    }
    fn to_error_code(&self) -> &'static str {
        match self {
            SearchDmMessagesError::Repository(e) => e.to_error_code(),
            SearchDmMessagesError::NotFriends => "NOT_FRIENDS",
            SearchDmMessagesError::EmptyQuery => "EMPTY_QUERY",
        }
    }
}

impl ToHttpStatus for MarkChannelReadError {
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

impl ToHttpStatus for MarkDmReadError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            MarkDmReadError::Repository(e) => e.to_http_status(),
            MarkDmReadError::NotFriends => StatusCode::FORBIDDEN,
        }
    }
    fn to_user_message(&self) -> String {
        match self {
            MarkDmReadError::Repository(e) => e.to_user_message(),
            MarkDmReadError::NotFriends => {
                "You are not friends with this user".to_string()
            }
        }
    }
    fn to_error_code(&self) -> &'static str {
        match self {
            MarkDmReadError::Repository(e) => e.to_error_code(),
            MarkDmReadError::NotFriends => "NOT_FRIENDS",
        }
    }
}

impl ToHttpStatus for GetUnreadCountsError {
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

impl ToHttpStatus for GetDmReadReceiptError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            GetDmReadReceiptError::Repository(e) => e.to_http_status(),
            GetDmReadReceiptError::NotFriends => StatusCode::FORBIDDEN,
        }
    }
    fn to_user_message(&self) -> String {
        match self {
            GetDmReadReceiptError::Repository(e) => e.to_user_message(),
            GetDmReadReceiptError::NotFriends => {
                "You are not friends with this user".to_string()
            }
        }
    }
    fn to_error_code(&self) -> &'static str {
        match self {
            GetDmReadReceiptError::Repository(e) => e.to_error_code(),
            GetDmReadReceiptError::NotFriends => "NOT_FRIENDS",
        }
    }
}

impl ToHttpStatus for GetChannelReadReceiptsError {
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

impl ToHttpStatus for ListFriendsError {
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

impl ToHttpStatus for ListChannelParticipantsError {
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
                "You are not a member of this server".to_string()
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
            ListChannelParticipantsError::NotMember => "SERVER_NOT_MEMBER",
            ListChannelParticipantsError::NotChannelMember => "CHANNEL_NOT_MEMBER",
            ListChannelParticipantsError::ChannelNotFound => "CHANNEL_NOT_FOUND",
        }
    }
}

impl ToHttpStatus for RoleError {
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
            RoleError::NotMember => "You are not a member of this server".to_string(),
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
            RoleError::NotMember => "SERVER_NOT_MEMBER",
            RoleError::NotAllowed => "ROLE_NOT_ALLOWED",
            RoleError::HierarchyViolation => "ROLE_HIERARCHY",
            RoleError::RoleNotFound => "ROLE_NOT_FOUND",
            RoleError::InvalidData => "ROLE_INVALID",
        }
    }
}

impl ToHttpStatus for SetNotificationPreferenceError {
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

shared::impl_to_http_status_delegate!(ListNotificationPreferencesError, Repository);

shared::impl_from_for_api_error!(
    AcceptFriendRequestError,
    AcceptServerInviteError,
    CreateChannelError,
    CreateInviteCodeError,
    CreateServerError,
    DeclineServerInviteError,
    DeleteChannelError,
    DeleteChannelMessageError,
    DeleteDmMessageError,
    DeleteServerError,
    EditChannelMessageError,
    EditDmMessageError,
    GetAttachmentError,
    GetChannelReadReceiptsError,
    GetDmReadReceiptError,
    GetUnreadCountsError,
    GetVoicePresenceError,
    JoinTextChannelError,
    JoinVoiceChannelError,
    KickMemberError,
    LeaveServerError,
    ListChannelMessagesError,
    ListChannelParticipantsError,
    ListDmMessagesError,
    ListDmPreviewsError,
    ListFriendsError,
    ListInviteCodesError,
    ListMessageAttachmentsError,
    ListNotificationPreferencesError,
    ListSentServerInvitesError,
    ListServerChannelsError,
    ListServerMembersError,
    ListServersByUserError,
    LogCallError,
    MarkChannelReadError,
    MarkDmReadError,
    MuteChannelError,
    RedeemInviteCodeError,
    RemoveFriendshipError,
    RevokeInviteCodeError,
    RoleError,
    SearchChannelMessagesError,
    SearchDmMessagesError,
    SendChannelMessageError,
    SendDmMessageError,
    SendFriendRequestError,
    SendServerInviteError,
    SetNotificationPreferenceError,
    UnmuteChannelError,
    UpdateServerError,
    UpdateStreamViewerError,
);
