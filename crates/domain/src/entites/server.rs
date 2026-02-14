use chrono::{DateTime, Utc};

use crate::value_objects::{
    server::{description::Description, id::ServerId, name::ServerName},
    user::id::UserId,
};

#[derive(Debug)]
pub struct Server {
    id: ServerId,
    name: ServerName,
    owner_id: UserId,
    description: Option<Description>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl Server {
    pub fn new(
        id: ServerId,
        name: ServerName,
        owner_id: UserId,
        description: Option<Description>,
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
}
