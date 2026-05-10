use chrono::{DateTime, Utc};

use crate::{
    channel::{
        channel_type::ChannelType,
        events::{
            ChannelEvent,
            lifecycle::{ChannelCreated, ChannelDeleted, ChannelRenamed},
            participant::ParticipantJoined,
        },
        id::ChannelId,
        max_paticipants::MaxParticipants,
        name::ChannelName,
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
}

#[derive(Debug)]
pub struct Channel {
    id: ChannelId,
    server_id: ServerId,
    owner_id: UserId,
    name: ChannelName,
    max_participants: MaxParticipants,
    channel_type: ChannelType,
    password_hash: Option<PasswordHash>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    last_join: DateTime<Utc>,
    events: Vec<ChannelEvent>,
}

impl Channel {
    pub fn create(
        server_id: ServerId,
        owner_id: UserId,
        name: ChannelName,
        channel_type: ChannelType,
        password_hash: Option<PasswordHash>,
        now: DateTime<Utc>,
    ) -> Result<Self, ChannelError> {
        if channel_type == ChannelType::Private && password_hash.is_none() {
            return Err(ChannelError::PrivateChannelRequiresPassword);
        }
        if channel_type == ChannelType::Public && password_hash.is_some() {
            return Err(ChannelError::PublicChannelCannotHavePassword);
        }

        let id = ChannelId::generate();
        let max_participants = MaxParticipants::default();

        let mut channel = Self {
            id,
            server_id,
            owner_id,
            name: name.clone(),
            max_participants,
            channel_type,
            password_hash,
            created_at: now,
            updated_at: now,
            last_join: now,
            events: Vec::new(),
        };

        channel.record(ChannelEvent::Created(ChannelCreated::record(
            id,
            server_id,
            owner_id,
            name,
            channel_type,
            now,
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
        channel_type: ChannelType,
        password_hash: Option<PasswordHash>,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
        last_join: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            server_id,
            owner_id,
            name,
            max_participants,
            channel_type,
            password_hash,
            created_at,
            updated_at,
            last_join,
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
        if self.channel_type != ChannelType::Private {
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

    pub fn channel_type(&self) -> &ChannelType {
        &self.channel_type
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
}
