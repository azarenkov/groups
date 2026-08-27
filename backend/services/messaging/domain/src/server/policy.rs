use std::collections::HashSet;

use uuid::Uuid;

use crate::{server::authorization::AuthorizationSnapshot, server::role::PERM_ALL, server::role::PERM_KICK_MEMBERS, server::role::PERM_MANAGE_CHANNELS, server::role::PERM_MANAGE_INVITES, server::role::PERM_MANAGE_MESSAGES, server::role::PERM_MANAGE_ROLES, server::role::PERM_MANAGE_SERVER, server::role::PERM_MUTE_MEMBERS, server::role::RolePatch};
use domain::user::id::UserId;

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

pub struct ModerationPolicy;

impl ModerationPolicy {
    fn ensure_member(snap: &AuthorizationSnapshot, actor: &UserId) -> Result<(), PolicyError> {
        if snap.is_member(actor) {
            Ok(())
        } else {
            Err(PolicyError::NotMember)
        }
    }

    fn ensure_permission(
        snap: &AuthorizationSnapshot,
        actor: &UserId,
        permission: i64,
    ) -> Result<(), PolicyError> {
        Self::ensure_member(snap, actor)?;
        if snap.has(actor, permission) {
            Ok(())
        } else {
            Err(PolicyError::NotAllowed)
        }
    }

    pub fn kick(snap: &AuthorizationSnapshot, actor: &UserId, target: &UserId) -> Result<(), PolicyError> {
        if actor == target || snap.is_owner(target) {
            return Err(PolicyError::NotAllowed);
        }
        Self::ensure_permission(snap, actor, PERM_KICK_MEMBERS)?;
        if !snap.is_member(target) {
            return Err(PolicyError::TargetNotMember);
        }
        if !snap.outranks_member(actor, target) {
            return Err(PolicyError::HierarchyViolation);
        }
        Ok(())
    }

    pub fn mute(snap: &AuthorizationSnapshot, actor: &UserId, target: &UserId) -> Result<(), PolicyError> {
        Self::ensure_member(snap, actor)?;
        if !snap.is_member(target) {
            return Err(PolicyError::TargetNotMember);
        }
        if actor == target {
            return Ok(());
        }
        if snap.is_owner(target) {
            return Err(PolicyError::NotAllowed);
        }
        Self::ensure_permission(snap, actor, PERM_MUTE_MEMBERS)?;
        if !snap.outranks_member(actor, target) {
            return Err(PolicyError::HierarchyViolation);
        }
        Ok(())
    }

    pub fn unmute(
        snap: &AuthorizationSnapshot,
        actor: &UserId,
        target: &UserId,
        muted_by: Option<&UserId>,
    ) -> Result<(), PolicyError> {
        Self::ensure_member(snap, actor)?;
        if snap.is_owner(actor) {
            return Ok(());
        }
        if actor == target {
            return if muted_by == Some(actor) {
                Ok(())
            } else {
                Err(PolicyError::NotAllowed)
            };
        }
        if muted_by.is_some_and(|by| snap.is_owner(by)) {
            return Err(PolicyError::NotAllowed);
        }
        Self::ensure_permission(snap, actor, PERM_MUTE_MEMBERS)?;
        if !snap.outranks_member(actor, target) {
            return Err(PolicyError::HierarchyViolation);
        }
        Ok(())
    }

    pub fn manage_channels(snap: &AuthorizationSnapshot, actor: &UserId) -> Result<(), PolicyError> {
        Self::ensure_permission(snap, actor, PERM_MANAGE_CHANNELS)
    }

    pub fn delete_message(
        snap: &AuthorizationSnapshot,
        actor: &UserId,
        author: &UserId,
    ) -> Result<(), PolicyError> {
        Self::ensure_member(snap, actor)?;
        if actor == author || snap.has(actor, PERM_MANAGE_MESSAGES) {
            Ok(())
        } else {
            Err(PolicyError::NotAllowed)
        }
    }

    pub fn manage_server(snap: &AuthorizationSnapshot, actor: &UserId) -> Result<(), PolicyError> {
        Self::ensure_permission(snap, actor, PERM_MANAGE_SERVER)
    }

    pub fn manage_invites(snap: &AuthorizationSnapshot, actor: &UserId) -> Result<(), PolicyError> {
        Self::ensure_permission(snap, actor, PERM_MANAGE_INVITES)
    }

    pub fn can_revoke_any_invite(snap: &AuthorizationSnapshot, actor: &UserId) -> bool {
        snap.is_member(actor) && snap.has(actor, PERM_MANAGE_INVITES)
    }
}
