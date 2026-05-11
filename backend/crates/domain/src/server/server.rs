use chrono::{DateTime, Utc};

use crate::{
    server::{description::ServerDescription, id::ServerId, name::ServerName},
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
        }
    }

    pub fn create(
        name: ServerName,
        owner_id: UserId,
        description: Option<ServerDescription>,
    ) -> Self {
        let now = Utc::now();

        Self {
            id: ServerId::generate(),
            name,
            owner_id,
            description,
            created_at: now,
            updated_at: now,
        }
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
