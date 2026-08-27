pub use domain::channel::{id, session_id, token};

pub mod events;
pub mod kind;
pub mod name;
pub mod participant_mute;
pub mod repository;
pub mod visibility;

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Utc};

use crate::{channel::events::ChannelEvent, channel::events::lifecycle::ChannelCreated, channel::events::lifecycle::ChannelDeleted, channel::events::lifecycle::ChannelRenamed, channel::events::participant::ConnectionAbortReason, channel::events::participant::LeaveReason, channel::events::participant::MediaSource, channel::events::participant::ParticipantConnectionAborted, channel::events::participant::ParticipantJoined, channel::events::participant::ParticipantLeft, channel::events::participant::ParticipantMediaChanged, channel::events::participant::ParticipantMuted, channel::events::participant::ParticipantUnmuted, channel::events::participant::TextChannelJoined, channel::id::ChannelId, channel::kind::ChannelKind, channel::name::ChannelName, channel::participant_mute::ParticipantMute, channel::session_id::SessionId, channel::visibility::ChannelVisibility, server::id::ServerId};
use domain::{shared::value_objects::password_hash::PasswordHash, user::id::UserId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnmuteOutcome {
    Removed,
    DowngradedToSelfMute,
}

#[derive(Debug, thiserror::Error)]
pub enum ChannelError {
    #[error("Private channel requires a password")]
    PrivateChannelRequiresPassword,
    #[error("Public channel cannot have a password")]
    PublicChannelCannotHavePassword,
    #[error("Channel name unchanged")]
    NameUnchanged,
    #[error("Channel deleting requires owner")]
    NotOwner,
    #[error("Participant not muted")]
    NotMuted,
    #[error("Participant already muted")]
    AlreadyMuted,
    #[error("Voice is not supported for this channel")]
    VoiceNotSupported,
    #[error("Text is not supported for this channel")]
    TextNotSupported,
    #[error("Participant has not joined the voice session yet")]
    ParticipantNotJoined,
}

#[derive(Debug, Clone)]
pub enum VoiceChange {
    Added {
        user_id: UserId,
        session_id: SessionId,
    },
    Removed {
        user_id: UserId,
        session_id: SessionId,
    },
}

#[derive(Debug, Clone)]
pub struct MediaChange {
    pub user_id: UserId,
    pub session_id: SessionId,
    pub source: MediaSource,
    pub enabled: bool,
}

#[derive(Debug)]
pub struct Channel {
    id: ChannelId,
    server_id: ServerId,
    owner_id: UserId,
    name: ChannelName,
    kind: ChannelKind,
    visibility: ChannelVisibility,
    password_hash: Option<PasswordHash>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    last_join: DateTime<Utc>,
    muted_participants: Option<HashMap<UserId, ParticipantMute>>,
    voice_participants: Option<HashMap<UserId, HashSet<SessionId>>>,
    camera_participants: Option<HashMap<UserId, HashSet<SessionId>>>,
    screen_participants: Option<HashMap<UserId, HashSet<SessionId>>>,
    text_participants: Option<HashSet<UserId>>,
    events: Vec<ChannelEvent>,
    voice_changes: Vec<VoiceChange>,
    media_changes: Vec<MediaChange>,
}

impl Channel {
    pub fn create(
        server_id: ServerId,
        owner_id: UserId,
        name: ChannelName,
        kind: ChannelKind,
        visibility: ChannelVisibility,
        password_hash: Option<PasswordHash>,
        now: DateTime<Utc>,
    ) -> Result<Self, ChannelError> {
        if visibility == ChannelVisibility::Private && password_hash.is_none() {
            return Err(ChannelError::PrivateChannelRequiresPassword);
        }
        if visibility == ChannelVisibility::Public && password_hash.is_some() {
            return Err(ChannelError::PublicChannelCannotHavePassword);
        }

        let id = ChannelId::generate();

        let (muted_participants, voice_participants, camera_participants, screen_participants) =
            if kind.supports_voice() {
                (
                    Some(HashMap::new()),
                    Some(HashMap::new()),
                    Some(HashMap::new()),
                    Some(HashMap::new()),
                )
            } else {
                (None, None, None, None)
            };

        let text_participants = if kind.supports_text() {
            let mut set = HashSet::new();
            set.insert(owner_id);
            Some(set)
        } else {
            None
        };

        let mut channel = Self {
            id,
            server_id,
            owner_id,
            name: name.clone(),
            kind,
            visibility,
            password_hash,
            created_at: now,
            updated_at: now,
            last_join: now,
            muted_participants,
            voice_participants,
            camera_participants,
            screen_participants,
            text_participants,
            events: Vec::new(),
            voice_changes: Vec::new(),
            media_changes: Vec::new(),
        };

        channel.record(ChannelEvent::Created(ChannelCreated::record(
            id, server_id, owner_id, name, kind, visibility, now,
        )));

        Ok(channel)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn restore(
        id: ChannelId,
        server_id: ServerId,
        owner_id: UserId,
        name: ChannelName,
        kind: ChannelKind,
        visibility: ChannelVisibility,
        password_hash: Option<PasswordHash>,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
        muted_participants: Option<HashMap<UserId, ParticipantMute>>,
        voice_participants: Option<HashMap<UserId, HashSet<SessionId>>>,
        camera_participants: Option<HashMap<UserId, HashSet<SessionId>>>,
        screen_participants: Option<HashMap<UserId, HashSet<SessionId>>>,
        text_participants: Option<HashSet<UserId>>,
        last_join: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            server_id,
            owner_id,
            name,
            kind,
            visibility,
            password_hash,
            created_at,
            updated_at,
            last_join,
            muted_participants,
            voice_participants,
            camera_participants,
            screen_participants,
            text_participants,
            events: Vec::new(),
            voice_changes: Vec::new(),
            media_changes: Vec::new(),
        }
    }

    pub fn rename(
        &mut self,
        new_name: ChannelName,
        now: DateTime<Utc>,
    ) -> Result<(), ChannelError> {
        if self.name == new_name {
            return Err(ChannelError::NameUnchanged);
        }

        let old_name = std::mem::replace(&mut self.name, new_name.clone());
        self.updated_at = now;

        self.record(ChannelEvent::Renamed(ChannelRenamed::record(
            self.id,
            self.server_id,
            old_name,
            new_name,
            now,
        )));

        Ok(())
    }

    pub fn delete(
        &mut self,
        deleted_by: UserId,
        can_override: bool,
        now: DateTime<Utc>,
    ) -> Result<(), ChannelError> {
        if self.owner_id != deleted_by && !can_override {
            return Err(ChannelError::NotOwner);
        }

        self.record(ChannelEvent::Deleted(ChannelDeleted::record(
            self.id,
            self.server_id,
            deleted_by,
            now,
        )));

        Ok(())
    }

    pub fn change_password(
        &mut self,
        new_hash: PasswordHash,
        now: DateTime<Utc>,
    ) -> Result<(), ChannelError> {
        if self.visibility != ChannelVisibility::Private {
            return Err(ChannelError::PublicChannelCannotHavePassword);
        }
        self.password_hash = Some(new_hash);
        self.updated_at = now;
        Ok(())
    }

    fn record(&mut self, event: ChannelEvent) {
        self.events.push(event);
    }

    pub fn pull_events(&mut self) -> Vec<ChannelEvent> {
        std::mem::take(&mut self.events)
    }

    pub fn pull_voice_changes(&mut self) -> Vec<VoiceChange> {
        std::mem::take(&mut self.voice_changes)
    }

    pub fn pull_media_changes(&mut self) -> Vec<MediaChange> {
        std::mem::take(&mut self.media_changes)
    }

    pub fn join(
        &mut self,
        user_id: UserId,
        session_id: SessionId,
        now: DateTime<Utc>,
    ) -> Result<(), ChannelError> {
        let participants = self
            .voice_participants
            .as_mut()
            .ok_or(ChannelError::VoiceNotSupported)?;

        let sessions = participants.entry(user_id).or_default();
        let was_empty = sessions.is_empty();

        if !sessions.insert(session_id.clone()) {
            return Ok(());
        }

        self.last_join = now;
        self.voice_changes.push(VoiceChange::Added {
            user_id,
            session_id,
        });

        if was_empty {
            self.record(ChannelEvent::ParticipantJoined(ParticipantJoined::record(
                self.id,
                self.server_id,
                user_id,
                now,
            )));
        }

        Ok(())
    }

    pub fn leave(
        &mut self,
        user_id: UserId,
        session_id: SessionId,
        reason: LeaveReason,
        now: DateTime<Utc>,
    ) -> Result<(), ChannelError> {
        let participants = self
            .voice_participants
            .as_mut()
            .ok_or(ChannelError::VoiceNotSupported)?;

        let Some(sessions) = participants.get_mut(&user_id) else {
            return Ok(());
        };

        if !sessions.remove(&session_id) {
            return Ok(());
        }

        let now_empty = sessions.is_empty();
        if now_empty {
            participants.remove(&user_id);
        }

        self.voice_changes.push(VoiceChange::Removed {
            user_id,
            session_id: session_id.clone(),
        });

        self.clear_session_media(user_id, &session_id, now);

        if now_empty {
            self.record(ChannelEvent::ParticipantLeft(ParticipantLeft::record(
                self.id,
                self.server_id,
                user_id,
                reason,
                now,
            )));
        }

        Ok(())
    }

    pub fn connection_aborted(
        &mut self,
        user_id: UserId,
        session_id: SessionId,
        reason: ConnectionAbortReason,
        now: DateTime<Utc>,
    ) -> Result<(), ChannelError> {
        let participants = self
            .voice_participants
            .as_mut()
            .ok_or(ChannelError::VoiceNotSupported)?;

        let Some(sessions) = participants.get_mut(&user_id) else {
            return Ok(());
        };

        if !sessions.remove(&session_id) {
            return Ok(());
        }

        let now_empty = sessions.is_empty();
        if now_empty {
            participants.remove(&user_id);
        }

        self.voice_changes.push(VoiceChange::Removed {
            user_id,
            session_id: session_id.clone(),
        });

        self.clear_session_media(user_id, &session_id, now);

        if now_empty {
            self.record(ChannelEvent::ParticipantConnectionAborted(
                ParticipantConnectionAborted::record(self.id, self.server_id, user_id, reason, now),
            ));
        }

        Ok(())
    }

    pub fn publish_track(
        &mut self,
        user_id: UserId,
        session_id: SessionId,
        source: MediaSource,
        now: DateTime<Utc>,
    ) -> Result<(), ChannelError> {
        self.set_track(user_id, session_id, source, true, now)
    }

    pub fn unpublish_track(
        &mut self,
        user_id: UserId,
        session_id: SessionId,
        source: MediaSource,
        now: DateTime<Utc>,
    ) -> Result<(), ChannelError> {
        self.set_track(user_id, session_id, source, false, now)
    }

    fn set_track(
        &mut self,
        user_id: UserId,
        session_id: SessionId,
        source: MediaSource,
        enabled: bool,
        now: DateTime<Utc>,
    ) -> Result<(), ChannelError> {
        let is_participant = self
            .voice_participants
            .as_ref()
            .ok_or(ChannelError::VoiceNotSupported)?
            .get(&user_id)
            .map(|sessions| sessions.contains(&session_id))
            .unwrap_or(false);
        if !is_participant {
            if enabled {
                return Err(ChannelError::ParticipantNotJoined);
            }
            return Ok(());
        }

        let transition = {
            let map = self
                .media_map_mut(source)
                .ok_or(ChannelError::VoiceNotSupported)?;
            let sessions = map.entry(user_id).or_default();
            let was_active = !sessions.is_empty();

            let changed = if enabled {
                sessions.insert(session_id.clone())
            } else {
                sessions.remove(&session_id)
            };
            if !changed {
                return Ok(());
            }

            let is_active = !sessions.is_empty();
            if !is_active {
                map.remove(&user_id);
            }
            (was_active, is_active)
        };

        self.media_changes.push(MediaChange {
            user_id,
            session_id,
            source,
            enabled,
        });

        let (was_active, is_active) = transition;
        if was_active != is_active {
            self.record(ChannelEvent::ParticipantMediaChanged(
                ParticipantMediaChanged::record(
                    self.id,
                    self.server_id,
                    user_id,
                    source,
                    is_active,
                    now,
                ),
            ));
        }

        Ok(())
    }

    fn media_map_mut(
        &mut self,
        source: MediaSource,
    ) -> Option<&mut HashMap<UserId, HashSet<SessionId>>> {
        match source {
            MediaSource::Camera => self.camera_participants.as_mut(),
            MediaSource::Screen => self.screen_participants.as_mut(),
        }
    }

    fn clear_session_media(
        &mut self,
        user_id: UserId,
        session_id: &SessionId,
        now: DateTime<Utc>,
    ) {
        for source in [MediaSource::Camera, MediaSource::Screen] {
            let became_inactive = {
                let Some(map) = self.media_map_mut(source) else {
                    continue;
                };
                let Some(sessions) = map.get_mut(&user_id) else {
                    continue;
                };
                if !sessions.remove(session_id) {
                    continue;
                }
                let empty = sessions.is_empty();
                if empty {
                    map.remove(&user_id);
                }
                empty
            };

            if became_inactive {
                self.record(ChannelEvent::ParticipantMediaChanged(
                    ParticipantMediaChanged::record(
                        self.id,
                        self.server_id,
                        user_id,
                        source,
                        false,
                        now,
                    ),
                ));
            }
        }
    }

    pub fn mute(
        &mut self,
        target: UserId,
        initiator: UserId,
        can_override: bool,
        now: DateTime<Utc>,
    ) -> Result<(), ChannelError> {
        if target != initiator && self.owner_id != initiator && !can_override {
            return Err(ChannelError::NotOwner);
        }
        let muted = self
            .muted_participants
            .as_mut()
            .ok_or(ChannelError::VoiceNotSupported)?;
        let mut self_muted = target == initiator;
        if let Some(existing) = muted.get(&target) {
            let replaces_self_mute =
                existing.muted_by == target && initiator != target;
            if !replaces_self_mute {
                return Err(ChannelError::AlreadyMuted);
            }
            self_muted = self_muted || existing.self_muted;
        }
        muted.insert(target, ParticipantMute::new(target, initiator, now, self_muted));

        self.record(ChannelEvent::ParticipantMuted(ParticipantMuted::record(
            self.id,
            self.server_id,
            target,
            initiator,
            now,
        )));

        Ok(())
    }

    pub fn unmute(
        &mut self,
        target: UserId,
        initiator: UserId,
        can_override: bool,
        now: DateTime<Utc>,
    ) -> Result<UnmuteOutcome, ChannelError> {
        let muted = self
            .muted_participants
            .as_ref()
            .ok_or(ChannelError::VoiceNotSupported)?;
        let mute_record = muted.get(&target).ok_or(ChannelError::NotMuted)?;

        let is_self_unmute_of_self_mute = target == initiator && mute_record.muted_by == initiator;
        let is_owner = self.owner_id == initiator;

        if !is_self_unmute_of_self_mute && !is_owner && !can_override {
            return Err(ChannelError::NotOwner);
        }

        let keeps_self_mute =
            mute_record.muted_by != target && mute_record.self_muted && initiator != target;
        if keeps_self_mute {
            self.muted_participants
                .as_mut()
                .ok_or(ChannelError::VoiceNotSupported)?
                .insert(target, ParticipantMute::new(target, target, now, true));
            self.record(ChannelEvent::ParticipantMuted(ParticipantMuted::record(
                self.id,
                self.server_id,
                target,
                target,
                now,
            )));
            return Ok(UnmuteOutcome::DowngradedToSelfMute);
        }

        self.muted_participants
            .as_mut()
            .ok_or(ChannelError::VoiceNotSupported)?
            .remove(&target);

        self.record(ChannelEvent::ParticipantUnmuted(
            ParticipantUnmuted::record(self.id, self.server_id, target, initiator, now),
        ));

        Ok(UnmuteOutcome::Removed)
    }

    pub fn muted_participants_list(&self) -> impl Iterator<Item = &ParticipantMute> {
        self.muted_participants.iter().flat_map(|m| m.values())
    }

    pub fn requires_password(&self) -> bool {
        self.password_hash.is_some()
    }

    pub fn id(&self) -> &ChannelId {
        &self.id
    }

    pub fn server_id(&self) -> &ServerId {
        &self.server_id
    }

    pub fn owner_id(&self) -> &UserId {
        &self.owner_id
    }

    pub fn name(&self) -> &ChannelName {
        &self.name
    }

    pub fn password_hash(&self) -> Option<&PasswordHash> {
        self.password_hash.as_ref()
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    pub fn last_join(&self) -> DateTime<Utc> {
        self.last_join
    }

    pub fn muted_participants(&self) -> &Option<HashMap<UserId, ParticipantMute>> {
        &self.muted_participants
    }

    pub fn visibility(&self) -> ChannelVisibility {
        self.visibility
    }

    pub fn kind(&self) -> &ChannelKind {
        &self.kind
    }

    pub fn supports_voice(&self) -> bool {
        self.kind.supports_voice()
    }

    pub fn supports_text(&self) -> bool {
        self.kind.supports_text()
    }

    pub fn join_text(
        &mut self,
        user_id: UserId,
        now: DateTime<Utc>,
    ) -> Result<bool, ChannelError> {
        let participants = self
            .text_participants
            .as_mut()
            .ok_or(ChannelError::TextNotSupported)?;

        if !participants.insert(user_id) {
            return Ok(false);
        }

        self.record(ChannelEvent::TextJoined(TextChannelJoined::record(
            self.id,
            self.server_id,
            user_id,
            now,
        )));

        Ok(true)
    }

    pub fn is_text_member(&self, user_id: &UserId) -> bool {
        self.text_participants
            .as_ref()
            .map(|p| p.contains(user_id))
            .unwrap_or(false)
    }

    pub fn text_participants(&self) -> Option<&HashSet<UserId>> {
        self.text_participants.as_ref()
    }
}
