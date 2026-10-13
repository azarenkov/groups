use std::time::Duration;

use domain::{channel::id::ChannelId, channel::token::ChannelToken, community::id::CommunityId, stream::{STREAM_CEILING_ATTRIBUTE, StreamCeiling}, user::id::UserId, user::name::UserName};
use ports::{voice_service::ParticipantAccess, voice_service::PublishedVideo, voice_service::VoiceService, voice_service::VoiceServiceError};
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
        ceiling: StreamCeiling,
    ) -> Result<ChannelToken, VoiceServiceError> {
        let token = AccessToken::with_api_key(self.config.api_key(), self.config.api_secret())
            .with_identity(&user_id.uuid().to_string())
            .with_name(username.as_str())
            .with_attributes([(STREAM_CEILING_ATTRIBUTE, ceiling.to_attribute())])
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
        ceiling: StreamCeiling,
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
            .with_attributes([(STREAM_CEILING_ATTRIBUTE, ceiling.to_attribute())])
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

    async fn set_stream_ceiling(
        &self,
        channel_id: &ChannelId,
        user_id: &UserId,
        ceiling: StreamCeiling,
    ) -> Result<(), VoiceServiceError> {
        let livekit_room = RoomClient::new(self.config.url())
            .map_err(|e| VoiceServiceError::ParticipantUpdate(e.to_string()))?;
        let attributes = std::collections::HashMap::from([(STREAM_CEILING_ATTRIBUTE.to_string(), ceiling.to_attribute())]);
        let (room, identity) = (channel_id.uuid().to_string(), user_id.uuid().to_string());
        let request = livekit_room.update_participant(
            &room,
            &identity,
            livekit_api::services::room::UpdateParticipantOptions { attributes, ..Default::default() },
        );
        match timeout(LIVEKIT_REQUEST_TIMEOUT, request).await {
            Ok(Ok(_)) => Ok(()),
            Ok(Err(ServiceError::Twirp(TwirpError::Twirp(code)))) if code.code == TwirpErrorCode::NOT_FOUND => Ok(()),
            Ok(Err(e)) => Err(VoiceServiceError::ParticipantUpdate(e.to_string())),
            Err(_) => Err(VoiceServiceError::ParticipantUpdate("LiveKit request timed out".into())),
        }
    }

    async fn screen_shares(
        &self,
        channel_id: &ChannelId,
        user_id: &UserId,
    ) -> Result<Vec<PublishedVideo>, VoiceServiceError> {
        let livekit_room = RoomClient::new(self.config.url())
            .map_err(|e| VoiceServiceError::ParticipantUpdate(e.to_string()))?;
        let (room, identity) = (channel_id.uuid().to_string(), user_id.uuid().to_string());
        let request = livekit_room.get_participant(&room, &identity);
        let participant = match timeout(LIVEKIT_REQUEST_TIMEOUT, request).await {
            Ok(Ok(participant)) => participant,
            Ok(Err(ServiceError::Twirp(TwirpError::Twirp(code)))) if code.code == TwirpErrorCode::NOT_FOUND => {
                return Ok(Vec::new());
            }
            Ok(Err(e)) => return Err(VoiceServiceError::ParticipantUpdate(e.to_string())),
            Err(_) => return Err(VoiceServiceError::ParticipantUpdate("LiveKit request timed out".into())),
        };
        Ok(participant
            .tracks
            .into_iter()
            .filter(|track| track.source == livekit_protocol::TrackSource::ScreenShare as i32 && !track.muted)
            .map(|track| published_video(&track))
            .collect())
    }

    async fn mute_track(
        &self,
        channel_id: &ChannelId,
        user_id: &UserId,
        track_sid: &str,
    ) -> Result<(), VoiceServiceError> {
        let livekit_room = RoomClient::new(self.config.url())
            .map_err(|e| VoiceServiceError::ParticipantUpdate(e.to_string()))?;
        let (room, identity) = (channel_id.uuid().to_string(), user_id.uuid().to_string());
        let request = livekit_room.mute_published_track(
            &room,
            &identity,
            track_sid,
            true,
        );
        match timeout(LIVEKIT_REQUEST_TIMEOUT, request).await {
            Ok(Ok(_)) => Ok(()),
            Ok(Err(ServiceError::Twirp(TwirpError::Twirp(code)))) if code.code == TwirpErrorCode::NOT_FOUND => Ok(()),
            Ok(Err(e)) => Err(VoiceServiceError::ParticipantUpdate(e.to_string())),
            Err(_) => Err(VoiceServiceError::ParticipantUpdate("LiveKit request timed out".into())),
        }
    }

    async fn active_rooms(&self) -> Result<Vec<ChannelId>, VoiceServiceError> {
        let livekit_room = RoomClient::new(self.config.url())
            .map_err(|e| VoiceServiceError::ParticipantUpdate(e.to_string()))?;
        let rooms = match timeout(LIVEKIT_REQUEST_TIMEOUT, livekit_room.list_rooms(Vec::new())).await {
            Ok(Ok(rooms)) => rooms,
            Ok(Err(e)) => return Err(VoiceServiceError::ParticipantUpdate(e.to_string())),
            Err(_) => return Err(VoiceServiceError::ParticipantUpdate("LiveKit request timed out".into())),
        };
        Ok(rooms
            .into_iter()
            .filter(|room| room.num_publishers > 0)
            .filter_map(|room| uuid::Uuid::parse_str(&room.name).ok().map(ChannelId::new))
            .collect())
    }

    async fn room_screen_shares(&self, channel_id: &ChannelId) -> Result<Vec<(UserId, PublishedVideo)>, VoiceServiceError> {
        let livekit_room = RoomClient::new(self.config.url())
            .map_err(|e| VoiceServiceError::ParticipantUpdate(e.to_string()))?;
        let room = channel_id.uuid().to_string();
        let participants = match timeout(LIVEKIT_REQUEST_TIMEOUT, livekit_room.list_participants(&room)).await {
            Ok(Ok(participants)) => participants,
            Ok(Err(ServiceError::Twirp(TwirpError::Twirp(code)))) if code.code == TwirpErrorCode::NOT_FOUND => {
                return Ok(Vec::new());
            }
            Ok(Err(e)) => return Err(VoiceServiceError::ParticipantUpdate(e.to_string())),
            Err(_) => return Err(VoiceServiceError::ParticipantUpdate("LiveKit request timed out".into())),
        };
        let mut shares = Vec::new();
        for participant in participants {
            let Ok(user) = uuid::Uuid::parse_str(&participant.identity) else {
                continue;
            };
            for track in &participant.tracks {
                if track.source == livekit_protocol::TrackSource::ScreenShare as i32 && !track.muted {
                    shares.push((UserId::new(user), published_video(track)));
                }
            }
        }
        Ok(shares)
    }

    fn client_url(&self) -> String {
        self.config.client_url.clone()
    }
}

const LIVEKIT_REQUEST_TIMEOUT: Duration = Duration::from_secs(5);

pub fn published_video(track: &livekit_protocol::TrackInfo) -> PublishedVideo {
    let layers = || track.codecs.iter().flat_map(|codec| codec.layers.iter());
    let top = layers().max_by_key(|layer| layer.width as u64 * layer.height as u64);
    PublishedVideo {
        sid: track.sid.clone(),
        width: top.map_or(track.width, |layer| layer.width.max(track.width)),
        height: top.map_or(track.height, |layer| layer.height.max(track.height)),
        bitrate: layers().map(|layer| layer.bitrate as u64).max().unwrap_or(0),
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

#[cfg(test)]
mod published_video_tests {
    use super::*;

    #[test]
    fn takes_the_largest_layer_and_its_bitrate() {
        let track = livekit_protocol::TrackInfo {
            sid: "TR_1".into(),
            width: 1280,
            height: 720,
            codecs: vec![livekit_protocol::SimulcastCodecInfo {
                layers: vec![
                    livekit_protocol::VideoLayer { width: 640, height: 360, bitrate: 500_000, ..Default::default() },
                    livekit_protocol::VideoLayer { width: 1920, height: 1080, bitrate: 6_000_000, ..Default::default() },
                ],
                ..Default::default()
            }],
            ..Default::default()
        };
        let video = published_video(&track);
        assert_eq!(video, PublishedVideo { sid: "TR_1".into(), width: 1920, height: 1080, bitrate: 6_000_000 });
    }

    #[test]
    fn falls_back_to_the_track_size_without_layers() {
        let track = livekit_protocol::TrackInfo { sid: "TR_2".into(), width: 3840, height: 2160, ..Default::default() };
        let video = published_video(&track);
        assert_eq!((video.width, video.height, video.bitrate), (3840, 2160, 0));
    }
}
