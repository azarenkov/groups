use std::collections::HashMap;

use chrono::{DateTime, Utc};

use crate::{
    channel::{
        events::{
            ChannelEvent,
            lifecycle::{ChannelCreated, ChannelDeleted, ChannelRenamed},
            participant::{
                ConnectionAbortReason, LeaveReason, ParticipantConnectionAborted,
                ParticipantJoined, ParticipantLeft, ParticipantMuted, ParticipantUnmuted,
            },
        },
        id::ChannelId,
        kind::ChannelKind,
        max_paticipants::MaxParticipants,
        name::ChannelName,
        participant_mute::ParticipantMute,
        visibility::ChannelVisibility,
    },
    server::id::ServerId,
    shared::value_objects::password_hash::PasswordHash,
    user::id::UserId,
};

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
}

#[derive(Debug)]
pub struct Channel {
    id: ChannelId,
    server_id: ServerId,
    owner_id: UserId,
    name: ChannelName,
    max_participants: MaxParticipants,
    kind: ChannelKind,
    visibility: ChannelVisibility,
    password_hash: Option<PasswordHash>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    last_join: DateTime<Utc>,
    muted_participants: Option<HashMap<UserId, ParticipantMute>>,
    events: Vec<ChannelEvent>,
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
        let max_participants = MaxParticipants::default();

        let muted_participants = if kind.supports_voice() {
            Some(HashMap::new())
        } else {
            None
        };

        let mut channel = Self {
            id,
            server_id,
            owner_id,
            name: name.clone(),
            max_participants,
            kind,
            visibility,
            password_hash,
            created_at: now,
            updated_at: now,
            last_join: now,
            muted_participants,
            events: Vec::new(),
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
        max_participants: MaxParticipants,
        kind: ChannelKind,
        visibility: ChannelVisibility,
        password_hash: Option<PasswordHash>,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
        muted_participants: Option<HashMap<UserId, ParticipantMute>>,
        last_join: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            server_id,
            owner_id,
            name,
            max_participants,
            kind,
            visibility,
            password_hash,
            created_at,
            updated_at,
            last_join,
            muted_participants,
            events: Vec::new(),
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

    pub fn delete(&mut self, deleted_by: UserId, now: DateTime<Utc>) -> Result<(), ChannelError> {
        if self.owner_id != deleted_by {
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

    pub fn change_max_participants(&mut self, new_max: MaxParticipants, now: DateTime<Utc>) {
        self.max_participants = new_max;
        self.updated_at = now;
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

    pub fn join(&mut self, user_id: UserId, now: DateTime<Utc>) -> Result<(), ChannelError> {
        self.last_join = now;

        self.record(ChannelEvent::ParticipantJoined(ParticipantJoined::record(
            self.id,
            self.server_id,
            user_id,
            now,
        )));

        Ok(())
    }

    pub fn leave(
        &mut self,
        user_id: UserId,
        reason: LeaveReason,
        now: DateTime<Utc>,
    ) -> Result<(), ChannelError> {
        self.record(ChannelEvent::ParticipantLeft(ParticipantLeft::record(
            self.id,
            self.server_id,
            user_id,
            reason,
            now,
        )));

        Ok(())
    }

    pub fn connection_aborted(
        &mut self,
        user_id: UserId,
        reason: ConnectionAbortReason,
        now: DateTime<Utc>,
    ) -> Result<(), ChannelError> {
        self.record(ChannelEvent::ParticipantConnectionAborted(
            ParticipantConnectionAborted::record(
                self.id,
                self.server_id,
                user_id,
                reason,
                now,
            ),
        ));

        Ok(())
    }

    pub fn mute(
        &mut self,
        target: UserId,
        initiator: UserId,
        now: DateTime<Utc>,
    ) -> Result<(), ChannelError> {
        if target != initiator && self.owner_id != initiator {
            return Err(ChannelError::NotOwner);
        }
        let muted = self
            .muted_participants
            .as_mut()
            .ok_or(ChannelError::VoiceNotSupported)?;
        if muted.contains_key(&target) {
            return Err(ChannelError::AlreadyMuted);
        }
        muted.insert(target, ParticipantMute::new(target, initiator, now));

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
        now: DateTime<Utc>,
    ) -> Result<(), ChannelError> {
        let muted = self
            .muted_participants
            .as_ref()
            .ok_or(ChannelError::VoiceNotSupported)?;
        let mute_record = muted.get(&target).ok_or(ChannelError::NotMuted)?;

        let is_self_unmute_of_self_mute = target == initiator && mute_record.muted_by == initiator;
        let is_owner = self.owner_id == initiator;

        if !is_self_unmute_of_self_mute && !is_owner {
            return Err(ChannelError::NotOwner);
        }

        self.muted_participants
            .as_mut()
            .ok_or(ChannelError::VoiceNotSupported)?
            .remove(&target);

        self.record(ChannelEvent::ParticipantUnmuted(
            ParticipantUnmuted::record(self.id, self.server_id, target, initiator, now),
        ));

        Ok(())
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

    pub fn max_participants(&self) -> &MaxParticipants {
        &self.max_participants
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
}
