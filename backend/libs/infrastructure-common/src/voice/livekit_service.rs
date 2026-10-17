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
    client: RoomClient,
    request_timeout: Duration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackMedia {
    Camera,
    Screen,
}

pub fn track_media(track: &livekit_protocol::TrackInfo) -> Option<TrackMedia> {
    match livekit_protocol::TrackSource::try_from(track.source).ok()? {
        livekit_protocol::TrackSource::Camera => Some(TrackMedia::Camera),
        livekit_protocol::TrackSource::ScreenShare => Some(TrackMedia::Screen),
        _ => None,
    }
}

#[derive(Debug, Clone)]
pub struct RoomMember {
    pub identity: String,
    pub session_id: String,
    pub camera_on: bool,
    pub screen_on: bool,
}

fn is_not_found(error: &ServiceError) -> bool {
    matches!(error, ServiceError::Twirp(TwirpError::Twirp(code)) if code.code == TwirpErrorCode::NOT_FOUND)
}

impl LiveKitService {
    pub fn new(config: LiveKitConfig) -> Self {
        let client = RoomClient::with_api_key(config.url(), config.api_key(), config.api_secret());
        Self { config, client, request_timeout: LIVEKIT_REQUEST_TIMEOUT }
    }

    pub fn with_request_timeout(mut self, request_timeout: Duration) -> Self {
        self.request_timeout = request_timeout;
        self
    }

    async fn call<T, F>(&self, request: F, failure: fn(String) -> VoiceServiceError) -> Result<T, VoiceServiceError>
    where
        F: std::future::Future<Output = Result<T, ServiceError>>,
    {
        match timeout(self.request_timeout, request).await {
            Ok(Ok(value)) => Ok(value),
            Ok(Err(e)) => Err(failure(e.to_string())),
            Err(_) => Err(failure("LiveKit request timed out".into())),
        }
    }

    async fn request<T, F>(&self, request: F, failure: fn(String) -> VoiceServiceError) -> Result<Option<T>, VoiceServiceError>
    where
        F: std::future::Future<Output = Result<T, ServiceError>>,
    {
        match timeout(self.request_timeout, request).await {
            Ok(Ok(value)) => Ok(Some(value)),
            Ok(Err(e)) if is_not_found(&e) => Ok(None),
            Ok(Err(e)) => Err(failure(e.to_string())),
            Err(_) => Err(failure("LiveKit request timed out".into())),
        }
    }

    async fn list_participants(&self, channel_id: &ChannelId) -> Result<Vec<livekit_protocol::ParticipantInfo>, VoiceServiceError> {
        let room = channel_id.uuid().to_string();
        Ok(self
            .request(self.client.list_participants(&room), VoiceServiceError::ParticipantUpdate)
            .await?
            .unwrap_or_default())
    }

    pub async fn room_members(&self, channel_id: &ChannelId) -> Result<Vec<RoomMember>, VoiceServiceError> {
        Ok(self
            .list_participants(channel_id)
            .await?
            .into_iter()
            .map(|p| RoomMember {
                camera_on: p.tracks.iter().any(|t| track_media(t) == Some(TrackMedia::Camera)),
                screen_on: p.tracks.iter().any(|t| track_media(t) == Some(TrackMedia::Screen)),
                identity: p.identity,
                session_id: p.sid,
            })
            .collect())
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
        let room = channel_id.uuid().to_string();
        let request = self.client.create_room(
            &room,
            CreateRoomOptions {
                metadata: community_id.uuid().to_string(),
                ..Default::default()
            },
        );
        self.request(request, VoiceServiceError::VoiceChannelCreation).await?;
        Ok(())
    }

    async fn delete_voice_channel(&self, channel_id: &ChannelId) -> Result<(), VoiceServiceError> {
        let room = channel_id.uuid().to_string();
        self.request(self.client.delete_room(&room), VoiceServiceError::VoiceChannelDeletion).await?;
        Ok(())
    }

    async fn remove_participant(
        &self,
        channel_id: &ChannelId,
        user_id: &UserId,
    ) -> Result<(), VoiceServiceError> {
        let (room, identity) = (channel_id.uuid().to_string(), user_id.uuid().to_string());
        let request = self.client.remove_participant(&room, &identity);
        self.request(request, VoiceServiceError::ParticipantRemoval).await?;
        Ok(())
    }

    async fn apply_participant_access(
        &self,
        channel_id: &ChannelId,
        user_id: &UserId,
        access: ParticipantAccess,
    ) -> Result<(), VoiceServiceError> {
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
        let request = self.client.update_participant(
            &room,
            &identity,
            livekit_api::services::room::UpdateParticipantOptions {
                permission: Some(permission),
                ..Default::default()
            },
        );
        let Some(participant) = self.request(request, VoiceServiceError::ParticipantRemoval).await? else {
            return Ok(());
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
            let request = self.client.mute_published_track(&room, &identity, &track.sid, true);
            self.request(request, VoiceServiceError::ParticipantRemoval).await?;
        }
        Ok(())
    }

    async fn set_stream_ceiling(
        &self,
        channel_id: &ChannelId,
        user_id: &UserId,
        ceiling: StreamCeiling,
    ) -> Result<(), VoiceServiceError> {
        let attributes = std::collections::HashMap::from([(STREAM_CEILING_ATTRIBUTE.to_string(), ceiling.to_attribute())]);
        let (room, identity) = (channel_id.uuid().to_string(), user_id.uuid().to_string());
        let request = self.client.update_participant(
            &room,
            &identity,
            livekit_api::services::room::UpdateParticipantOptions { attributes, ..Default::default() },
        );
        self.request(request, VoiceServiceError::ParticipantUpdate).await?;
        Ok(())
    }

    async fn mute_track(
        &self,
        channel_id: &ChannelId,
        user_id: &UserId,
        track_sid: &str,
    ) -> Result<(), VoiceServiceError> {
        let (room, identity) = (channel_id.uuid().to_string(), user_id.uuid().to_string());
        let request = self.client.mute_published_track(&room, &identity, track_sid, true);
        self.request(request, VoiceServiceError::ParticipantUpdate).await?;
        Ok(())
    }

    async fn active_rooms(&self) -> Result<Vec<ChannelId>, VoiceServiceError> {
        let rooms = self.call(self.client.list_rooms(Vec::new()), VoiceServiceError::ParticipantUpdate).await?;
        Ok(rooms
            .into_iter()
            .filter(|room| room.num_participants > 0)
            .filter_map(|room| uuid::Uuid::parse_str(&room.name).ok().map(ChannelId::new))
            .collect())
    }

    async fn room_participants(&self, channel_id: &ChannelId) -> Result<Vec<UserId>, VoiceServiceError> {
        Ok(self
            .list_participants(channel_id)
            .await?
            .iter()
            .filter_map(|participant| uuid::Uuid::parse_str(&participant.identity).ok().map(UserId::new))
            .collect())
    }

    async fn room_videos(&self, channel_id: &ChannelId) -> Result<Vec<(UserId, PublishedVideo)>, VoiceServiceError> {
        let mut shares = Vec::new();
        for participant in self.list_participants(channel_id).await? {
            let Ok(user) = uuid::Uuid::parse_str(&participant.identity) else {
                continue;
            };
            for track in &participant.tracks {
                if track.r#type == livekit_protocol::TrackType::Video as i32 && !track.muted {
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
        bitrate: track.codecs.iter().map(codec_bitrate).max().unwrap_or(0),
    }
}

fn codec_bitrate(codec: &livekit_protocol::SimulcastCodecInfo) -> u64 {
    let layers = codec.layers.iter().map(|layer| layer.bitrate as u64);
    if codec.video_layer_mode == livekit_protocol::video_layer::Mode::MultipleSpatialLayersPerStream as i32 {
        layers.max().unwrap_or(0)
    } else {
        layers.sum()
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
        assert_eq!(video, PublishedVideo { sid: "TR_1".into(), width: 1920, height: 1080, bitrate: 6_500_000 });
    }

    #[test]
    fn counts_svc_layers_once_and_the_active_codec_only() {
        let layer = |width, height, bitrate| livekit_protocol::VideoLayer { width, height, bitrate, ..Default::default() };
        let track = livekit_protocol::TrackInfo {
            sid: "TR_3".into(),
            codecs: vec![
                livekit_protocol::SimulcastCodecInfo {
                    video_layer_mode: livekit_protocol::video_layer::Mode::MultipleSpatialLayersPerStream as i32,
                    layers: vec![layer(640, 360, 1_000_000), layer(1280, 720, 3_000_000)],
                    ..Default::default()
                },
                livekit_protocol::SimulcastCodecInfo {
                    layers: vec![layer(640, 360, 500_000), layer(1280, 720, 2_000_000)],
                    ..Default::default()
                },
            ],
            ..Default::default()
        };
        assert_eq!(published_video(&track).bitrate, 3_000_000);
    }

    #[test]
    fn falls_back_to_the_track_size_without_layers() {
        let track = livekit_protocol::TrackInfo { sid: "TR_2".into(), width: 3840, height: 2160, ..Default::default() };
        let video = published_video(&track);
        assert_eq!((video.width, video.height, video.bitrate), (3840, 2160, 0));
    }
}
