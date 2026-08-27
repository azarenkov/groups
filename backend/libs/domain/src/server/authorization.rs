use std::collections::HashSet;

use uuid::Uuid;

use crate::{
    server::role::{PERM_ALL, ServerRole, has_permission},
    user::id::UserId,
};

#[derive(Debug, Clone)]
pub struct AuthorizationSnapshot {
    pub owner_id: UserId,
    pub roles: Vec<ServerRole>,
    pub assignments: Vec<(UserId, Uuid)>,
    pub members: HashSet<UserId>,
}

impl AuthorizationSnapshot {
    pub fn is_owner(&self, user: &UserId) -> bool {
        &self.owner_id == user
    }

    pub fn is_member(&self, user: &UserId) -> bool {
        self.members.contains(user)
    }

    pub fn role(&self, role_id: Uuid) -> Option<&ServerRole> {
        self.roles.iter().find(|r| r.id == role_id)
    }

    pub fn roles_of<'a>(&'a self, user: &'a UserId) -> impl Iterator<Item = &'a ServerRole> + 'a {
        self.assignments
            .iter()
            .filter(move |(u, _)| u == user)
            .filter_map(move |(_, rid)| self.role(*rid))
    }

    pub fn permissions_of(&self, user: &UserId) -> i64 {
        if self.is_owner(user) {
            return PERM_ALL;
        }
        if !self.is_member(user) {
            return 0;
        }
        self.roles_of(user).fold(0, |acc, r| acc | r.permissions)
    }

    pub fn has(&self, user: &UserId, permission: i64) -> bool {
        has_permission(self.permissions_of(user), permission)
    }

    pub fn top_position(&self, user: &UserId) -> Option<i32> {
        if !self.is_member(user) {
            return None;
        }
        self.roles_of(user).map(|r| r.position).max()
    }

    pub fn holders(&self, role_id: Uuid) -> Vec<UserId> {
        self.assignments
            .iter()
            .filter(|(u, rid)| *rid == role_id && self.is_member(u))
            .map(|(u, _)| *u)
            .collect()
    }

    pub fn outranks_position(&self, actor: &UserId, position: i32) -> bool {
        if self.is_owner(actor) {
            return true;
        }
        matches!(self.top_position(actor), Some(top) if position < top)
    }

    pub fn outranks_member(&self, actor: &UserId, target: &UserId) -> bool {
        if self.is_owner(actor) {
            return true;
        }
        if self.is_owner(target) {
            return false;
        }
        match (self.top_position(actor), self.top_position(target)) {
            (Some(a), Some(t)) => a > t,
            (Some(_), None) => true,
            _ => false,
        }
    }

    pub fn grantable(&self, actor: &UserId) -> i64 {
        self.permissions_of(actor)
    }

    pub fn taken_positions(&self) -> Vec<i32> {
        self.roles.iter().map(|r| r.position).collect()
    }
}
