pub mod description;
pub mod events;
pub mod id;
pub mod name;
pub mod repository;

use chrono::{DateTime, Utc};

use crate::{
    server::{
        description::ServerDescription,
        events::{
            ServerEvent,
            lifecycle::{ServerCreated, ServerDeleted},
        },
        id::ServerId,
        name::ServerName,
    },
    user::id::UserId,
};

#[derive(Debug)]
pub struct Server {
    id: ServerId,
    name: ServerName,
    owner_id: UserId,
    description: Option<ServerDescription>,
    accent_color: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    events: Vec<ServerEvent>,
}

impl Server {
    pub fn restore(
        id: ServerId,
        name: ServerName,
        owner_id: UserId,
        description: Option<ServerDescription>,
        accent_color: Option<String>,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            name,
            owner_id,
            description,
            accent_color,
            created_at,
            updated_at,
            events: Vec::new(),
        }
    }

    pub fn create(
        name: ServerName,
        owner_id: UserId,
        description: Option<ServerDescription>,
        accent_color: Option<String>,
    ) -> Self {
        let now = Utc::now();
        let id = ServerId::generate();
        let event = ServerEvent::Created(ServerCreated::record(
            id,
            owner_id,
            name.clone(),
            description.clone(),
            now,
        ));

        Self {
            id,
            name,
            owner_id,
            description,
            accent_color,
            created_at: now,
            updated_at: now,
            events: vec![event],
        }
    }

    pub fn delete(&mut self, member_ids: Vec<UserId>, now: DateTime<Utc>) {
        self.record(ServerEvent::Deleted(ServerDeleted::record(
            self.id,
            self.owner_id,
            member_ids,
            now,
        )));
    }

    fn record(&mut self, event: ServerEvent) {
        self.events.push(event);
    }

    pub fn pull_events(&mut self) -> Vec<ServerEvent> {
        std::mem::take(&mut self.events)
    }

    pub fn id(&self) -> &ServerId {
        &self.id
    }

    pub fn name(&self) -> &ServerName {
        &self.name
    }

    pub fn owner_id(&self) -> &UserId {
        &self.owner_id
    }

    pub fn description(&self) -> Option<&ServerDescription> {
        self.description.as_ref()
    }

    pub fn accent_color(&self) -> Option<&str> {
        self.accent_color.as_deref()
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }
}
