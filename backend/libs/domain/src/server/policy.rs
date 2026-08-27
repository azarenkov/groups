use std::collections::HashSet;

use uuid::Uuid;

use crate::{
    server::{
        authorization::AuthorizationSnapshot,
        role::{PERM_ALL, PERM_MANAGE_ROLES, RolePatch},
    },
    user::id::UserId,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyError {
    NotMember,
    NotAllowed,
    HierarchyViolation,
    RoleNotFound,
    TargetNotMember,
    LimitReached,
}

pub const MAX_ROLES_PER_SERVER: usize = 50;

pub struct RolePolicy;

impl RolePolicy {
    pub fn ensure_manager(snap: &AuthorizationSnapshot, actor: &UserId) -> Result<(), PolicyError> {
        if !snap.is_member(actor) {
            return Err(PolicyError::NotMember);
        }
        if snap.is_owner(actor) || snap.has(actor, PERM_MANAGE_ROLES) {
            return Ok(());
        }
        Err(PolicyError::NotAllowed)
    }

    fn ensure_grantable(snap: &AuthorizationSnapshot, actor: &UserId, bits: i64) -> Result<(), PolicyError> {
        if bits & !snap.grantable(actor) != 0 {
            return Err(PolicyError::NotAllowed);
        }
        Ok(())
    }

    fn ensure_outranks_holders(
        snap: &AuthorizationSnapshot,
        actor: &UserId,
        role_id: Uuid,
    ) -> Result<(), PolicyError> {
        if snap.is_owner(actor) {
            return Ok(());
        }
        for holder in snap.holders(role_id) {
            if snap.is_owner(&holder) || !snap.outranks_member(actor, &holder) {
                return Err(PolicyError::HierarchyViolation);
            }
        }
        Ok(())
    }

    pub fn create(
        snap: &AuthorizationSnapshot,
        actor: &UserId,
        permissions: i64,
    ) -> Result<i32, PolicyError> {
        Self::ensure_manager(snap, actor)?;
        if snap.roles.len() >= MAX_ROLES_PER_SERVER {
            return Err(PolicyError::LimitReached);
        }
        Self::ensure_grantable(snap, actor, permissions & PERM_ALL)?;
        let taken: HashSet<i32> = snap.taken_positions().into_iter().collect();
        if snap.is_owner(actor) {
            return Ok(taken.iter().copied().max().unwrap_or(0) + 1);
        }
        let Some(top) = snap.top_position(actor) else {
            return Err(PolicyError::HierarchyViolation);
        };
        let mut candidate = top - 1;
        while taken.contains(&candidate) {
            candidate -= 1;
        }
        Ok(candidate)
    }

    pub fn update(
        snap: &AuthorizationSnapshot,
        actor: &UserId,
        role_id: Uuid,
        patch: &RolePatch,
    ) -> Result<i64, PolicyError> {
        Self::ensure_manager(snap, actor)?;
        let role = snap.role(role_id).ok_or(PolicyError::RoleNotFound)?;
        if !snap.outranks_position(actor, role.position) {
            return Err(PolicyError::HierarchyViolation);
        }
        let projected =
            (role.permissions | patch.add_permissions) & !patch.remove_permissions & PERM_ALL;
        Self::ensure_grantable(snap, actor, projected & !role.permissions)?;
        if role.permissions & !projected != 0 {
            Self::ensure_outranks_holders(snap, actor, role_id)?;
        }
        Ok(projected)
    }

    pub fn delete(snap: &AuthorizationSnapshot, actor: &UserId, role_id: Uuid) -> Result<(), PolicyError> {
        Self::ensure_manager(snap, actor)?;
        let role = snap.role(role_id).ok_or(PolicyError::RoleNotFound)?;
        if !snap.outranks_position(actor, role.position) {
            return Err(PolicyError::HierarchyViolation);
        }
        Self::ensure_outranks_holders(snap, actor, role_id)
    }

    pub fn assign(
        snap: &AuthorizationSnapshot,
        actor: &UserId,
        target: &UserId,
        role_id: Uuid,
    ) -> Result<(), PolicyError> {
        Self::ensure_manager(snap, actor)?;
        let role = snap.role(role_id).ok_or(PolicyError::RoleNotFound)?;
        if !snap.is_member(target) {
            return Err(PolicyError::TargetNotMember);
        }
        if snap.is_owner(actor) {
            return Ok(());
        }
        if !snap.outranks_position(actor, role.position) {
            return Err(PolicyError::HierarchyViolation);
        }
        Self::ensure_grantable(snap, actor, role.permissions)?;
        if !snap.outranks_member(actor, target) {
            return Err(PolicyError::HierarchyViolation);
        }
        Ok(())
    }

    pub fn revoke(
        snap: &AuthorizationSnapshot,
        actor: &UserId,
        target: &UserId,
        role_id: Uuid,
    ) -> Result<(), PolicyError> {
        Self::ensure_manager(snap, actor)?;
        let role = snap.role(role_id).ok_or(PolicyError::RoleNotFound)?;
        if snap.is_owner(actor) {
            return Ok(());
        }
        if !snap.outranks_position(actor, role.position) || !snap.outranks_member(actor, target) {
            return Err(PolicyError::HierarchyViolation);
        }
        Ok(())
    }

    pub fn swap(
        snap: &AuthorizationSnapshot,
        actor: &UserId,
        role_a: Uuid,
        role_b: Uuid,
    ) -> Result<(), PolicyError> {
        Self::ensure_manager(snap, actor)?;
        let a = snap.role(role_a).ok_or(PolicyError::RoleNotFound)?;
        let b = snap.role(role_b).ok_or(PolicyError::RoleNotFound)?;
        if !snap.outranks_position(actor, a.position) || !snap.outranks_position(actor, b.position) {
            return Err(PolicyError::HierarchyViolation);
        }
        Ok(())
    }
}
