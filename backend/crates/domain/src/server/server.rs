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
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            name,
            owner_id,
            description,
            created_at,
            updated_at,
            events: Vec::new(),
        }
    }

    pub fn create(
        name: ServerName,
        owner_id: UserId,
        description: Option<ServerDescription>,
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
            created_at: now,
            updated_at: now,
            events: vec![event],
        }
    }

    pub fn delete(&mut self, now: DateTime<Utc>) {
        self.record(ServerEvent::Deleted(ServerDeleted::record(
            self.id,
            self.owner_id,
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

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }
}
