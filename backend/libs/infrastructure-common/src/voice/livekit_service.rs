use std::time::Duration;

use domain::{channel::id::ChannelId, channel::token::ChannelToken, community::id::CommunityId, user::id::UserId, user::name::UserName};
use ports::{voice_service::ParticipantAccess, voice_service::VoiceService, voice_service::VoiceServiceError};
use async_trait::async_trait;

use livekit_api::{
    access_token::{self, AccessToken},
    services::{
        ServiceError, TwirpError, TwirpErrorCode,
        room::{CreateRoomOptions, RoomClient},
    },
};
use tokio::{net::TcpStream, time::timeout};

use crate::config::livekit::LiveKitConfig;

pub struct LiveKitService {
    config: LiveKitConfig,
}

impl LiveKitService {
    pub fn new(config: LiveKitConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl VoiceService for LiveKitService {
    async fn create_token(
        &self,
        user_id: &UserId,
        username: &UserName,
        channel_id: &ChannelId,
    ) -> Result<ChannelToken, VoiceServiceError> {
        let token = AccessToken::with_api_key(self.config.api_key(), self.config.api_secret())
            .with_identity(&user_id.uuid().to_string())
            .with_name(username.as_str())
            .with_grants(access_token::VideoGrants {
                room_join: true,
                room: channel_id.uuid().to_string(),
                can_update_own_metadata: true,
                ..Default::default()
            })
            .with_ttl(Duration::from_secs(3600))
            .to_jwt()
            .map_err(|e| VoiceServiceError::VoiceChannelCreation(e.to_string()))?;

        Ok(ChannelToken::new(token))
    }

    async fn create_token_with_access(
        &self,
        user_id: &UserId,
        username: &UserName,
        channel_id: &ChannelId,
        access: ParticipantAccess,
    ) -> Result<ChannelToken, VoiceServiceError> {
        let can_publish_sources = if access.can_publish_microphone {
            Vec::new()
        } else {
            vec![
                "camera".to_string(),
                "screen_share".to_string(),
                "screen_share_audio".to_string(),
            ]
        };
        let token = AccessToken::with_api_key(self.config.api_key(), self.config.api_secret())
            .with_identity(&user_id.uuid().to_string())
            .with_name(username.as_str())
            .with_grants(access_token::VideoGrants {
                room_join: true,
                room: channel_id.uuid().to_string(),
                can_subscribe: access.can_subscribe,
                can_publish_sources,
                can_update_own_metadata: true,
                ..Default::default()
            })
            .with_ttl(Duration::from_secs(3600))
            .to_jwt()
            .map_err(|e| VoiceServiceError::VoiceChannelCreation(e.to_string()))?;

        Ok(ChannelToken::new(token))
    }

    async fn create_voice_channel(
        &self,
        channel_id: &ChannelId,
        community_id: &CommunityId,
    ) -> Result<(), VoiceServiceError> {
        let livekit_room = RoomClient::new(self.config.url())
            .map_err(|e| VoiceServiceError::VoiceChannelCreation(e.to_string()))?;

        livekit_room
            .create_room(
                &channel_id.uuid().to_string(),
                CreateRoomOptions {
                    metadata: community_id.uuid().to_string(),
                    ..Default::default()
                },
            )
            .await
            .map_err(|e| VoiceServiceError::VoiceChannelCreation(e.to_string()))?;

        Ok(())
    }

    async fn delete_voice_channel(&self, channel_id: &ChannelId) -> Result<(), VoiceServiceError> {
        let livekit_room = RoomClient::new(self.config.url())
            .map_err(|e| VoiceServiceError::VoiceChannelDeletion(e.to_string()))?;

        match livekit_room
            .delete_room(&channel_id.uuid().to_string())
            .await
        {
            Ok(_) => Ok(()),
            Err(ServiceError::Twirp(TwirpError::Twirp(code)))
                if code.code == TwirpErrorCode::NOT_FOUND =>
            {
                Ok(())
            }
            Err(e) => Err(VoiceServiceError::VoiceChannelDeletion(e.to_string())),
        }
    }

    async fn remove_participant(
        &self,
        channel_id: &ChannelId,
        user_id: &UserId,
    ) -> Result<(), VoiceServiceError> {
        let livekit_room = RoomClient::new(self.config.url())
            .map_err(|e| VoiceServiceError::ParticipantRemoval(e.to_string()))?;

        match livekit_room
            .remove_participant(
                &channel_id.uuid().to_string(),
                &user_id.uuid().to_string(),
            )
            .await
        {
            Ok(_) => Ok(()),
            Err(ServiceError::Twirp(TwirpError::Twirp(code)))
                if code.code == TwirpErrorCode::NOT_FOUND =>
            {
                Ok(())
            }
            Err(e) => Err(VoiceServiceError::ParticipantRemoval(e.to_string())),
        }
    }

    async fn apply_participant_access(
        &self,
        channel_id: &ChannelId,
        user_id: &UserId,
        access: ParticipantAccess,
    ) -> Result<(), VoiceServiceError> {
        let livekit_room = RoomClient::new(self.config.url())
            .map_err(|e| VoiceServiceError::ParticipantRemoval(e.to_string()))?;
        let room = channel_id.uuid().to_string();
        let identity = user_id.uuid().to_string();
        let can_publish_sources = if access.can_publish_microphone {
            Vec::new()
        } else {
            vec![
                livekit_protocol::TrackSource::Camera as i32,
                livekit_protocol::TrackSource::ScreenShare as i32,
                livekit_protocol::TrackSource::ScreenShareAudio as i32,
            ]
        };
        let permission = livekit_protocol::ParticipantPermission {
            can_subscribe: access.can_subscribe,
            can_publish: true,
            can_publish_data: true,
            can_publish_sources,
            can_update_metadata: true,
            ..Default::default()
        };
        let participant = match livekit_room
            .update_participant(
                &room,
                &identity,
                livekit_api::services::room::UpdateParticipantOptions {
                    permission: Some(permission),
                    ..Default::default()
                },
            )
            .await
        {
            Ok(p) => p,
            Err(ServiceError::Twirp(TwirpError::Twirp(code)))
                if code.code == TwirpErrorCode::NOT_FOUND =>
            {
                return Ok(());
            }
            Err(e) => return Err(VoiceServiceError::ParticipantRemoval(e.to_string())),
        };
        if access.can_publish_microphone {
            return Ok(());
        }
        for track in participant.tracks {
            let is_live_microphone =
                track.source == livekit_protocol::TrackSource::Microphone as i32 && !track.muted;
            if !is_live_microphone {
                continue;
            }
            match livekit_room
                .mute_published_track(&room, &identity, &track.sid, true)
                .await
            {
                Ok(_) => {}
                Err(ServiceError::Twirp(TwirpError::Twirp(code)))
                    if code.code == TwirpErrorCode::NOT_FOUND => {}
                Err(e) => return Err(VoiceServiceError::ParticipantRemoval(e.to_string())),
            }
        }
        Ok(())
    }

    fn client_url(&self) -> String {
        self.config.client_url.clone()
    }
}

pub async fn is_livekit_reachable(url: &str) -> bool {
    let Some(host_port) = parse_host_port(url) else {
        return false;
    };
    matches!(
        timeout(
            Duration::from_secs(2),
            TcpStream::connect(host_port.as_str())
        )
        .await,
        Ok(Ok(_))
    )
}

fn parse_host_port(url: &str) -> Option<String> {
    let without_scheme = url
        .strip_prefix("http://")
        .or_else(|| url.strip_prefix("https://"))
        .or_else(|| url.strip_prefix("ws://"))
        .or_else(|| url.strip_prefix("wss://"))
        .unwrap_or(url);
    let host_port = without_scheme.split('/').next()?.split('?').next()?;
    if host_port.is_empty() {
        return None;
    }

    let has_port = if let Some(after_bracket) = host_port.strip_prefix('[') {
        after_bracket
            .rsplit_once(']')
            .map(|(_, rest)| rest.starts_with(':') && rest.len() > 1)
            .unwrap_or(false)
    } else {
        host_port.contains(':')
    };

    if has_port {
        Some(host_port.to_string())
    } else {
        let default_port = if url.starts_with("https://") || url.starts_with("wss://") {
            443
        } else {
            80
        };
        Some(format!("{host_port}:{default_port}"))
    }
}
