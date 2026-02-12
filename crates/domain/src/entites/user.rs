use chrono::{DateTime, Utc};

use crate::value_objects::{
    password_hash::PasswordHash,
    user::{email::Email, id::UserId, name::UserName},
};

#[derive(Debug)]
pub struct User {
    id: UserId,
    name: UserName,
    email: Email,
    password_hash: PasswordHash,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl User {
    pub fn new(
        id: UserId,
        name: UserName,
        email: Email,
        password_hash: PasswordHash,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            name,
            email,
            password_hash,
            created_at,
            updated_at,
        }
    }

    pub fn id(&self) -> &UserId {
        &self.id
    }

    pub fn name(&self) -> &UserName {
        &self.name
    }
}
