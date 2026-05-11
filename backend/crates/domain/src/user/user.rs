use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::{
    shared::value_objects::password_hash::PasswordHash,
    user::{email::Email, id::UserId, name::UserName},
};

#[derive(Debug, Serialize)]
pub struct User {
    id: UserId,
    name: UserName,
    email: Email,
    password_hash: PasswordHash,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    email_verified_at: Option<DateTime<Utc>>,
    phone_verified_at: Option<DateTime<Utc>>,
}

impl User {
    pub fn new(
        id: UserId,
        name: UserName,
        email: Email,
        password_hash: PasswordHash,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
        email_verified_at: Option<DateTime<Utc>>,
        phone_verified_at: Option<DateTime<Utc>>,
    ) -> Self {
        Self {
            id,
            name,
            email,
            password_hash,
            created_at,
            updated_at,
            email_verified_at,
            phone_verified_at,
        }
    }

    pub fn create(name: UserName, email: Email, password_hash: PasswordHash) -> Self {
        let now = Utc::now();
        Self {
            id: UserId::generate(),
            name,
            email,
            password_hash,
            created_at: now,
            updated_at: now,
            email_verified_at: None,
            phone_verified_at: None,
        }
    }

    pub fn is_verified(&self) -> bool {
        self.email_verified_at.is_some() || self.phone_verified_at.is_some()
    }

    pub fn id(&self) -> &UserId {
        &self.id
    }

    pub fn id_ownned(&self) -> UserId {
        self.id
    }

    pub fn name(&self) -> &UserName {
        &self.name
    }

    pub fn password_hash(&self) -> &PasswordHash {
        &self.password_hash
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    pub fn email(&self) -> &Email {
        &self.email
    }

    pub fn email_verified_at(&self) -> Option<DateTime<Utc>> {
        self.email_verified_at
    }

    pub fn phone_verified_at(&self) -> Option<DateTime<Utc>> {
        self.phone_verified_at
    }

    pub fn set_email_verified_at(&mut self, email_verified_at: Option<DateTime<Utc>>) {
        self.email_verified_at = email_verified_at;
    }

    pub fn set_phone_verified_at(&mut self, phone_verified_at: Option<DateTime<Utc>>) {
        self.phone_verified_at = phone_verified_at;
    }
}
