use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    id: Uuid,
    nickname: String,
    email: String,
    password_hash: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl User {
    pub fn new(nickname: String, email: String, password_hash: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            nickname,
            email,
            password_hash,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn nickname(&self) -> &str {
        &self.nickname
    }

    pub fn email(&self) -> &str {
        &self.email
    }

    pub fn verify_password(&self, password_hash: &str) -> bool {
        self.password_hash == password_hash
    }

    pub fn update_nickname(&mut self, nickname: String) {
        self.nickname = nickname;
        self.updated_at = Utc::now();
    }
}
