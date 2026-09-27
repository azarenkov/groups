pub use domain::community::id;

pub mod description;
pub mod events;
pub mod invite_link;
pub mod name;
pub mod policy;
pub mod repository;
pub mod role;
pub mod unit_of_work;

#[cfg(test)]
mod tests;

use std::collections::HashSet;

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::{channel::moderation::ModerationGrant, community::description::CommunityDescription, community::events::CommunityEvent, community::events::lifecycle::CommunityCreated, community::events::lifecycle::CommunityDeleted, community::events::lifecycle::CommunityOwnerChanged, community::events::lifecycle::CommunityMemberLeft, community::events::lifecycle::CommunityRolesChanged, community::events::lifecycle::CommunityUpdated, community::id::CommunityId, community::name::CommunityName, community::policy::MAX_ROLES_PER_COMMUNITY, community::policy::PolicyError, community::role::Permission, community::role::Permissions, community::role::RolePatch, community::role::CommunityRole};
use domain::user::id::UserId;

#[derive(Debug)]
pub struct Community {
    id: CommunityId,
    name: CommunityName,
    owner_id: UserId,
    description: Option<CommunityDescription>,
    accent_color: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    roles: Vec<CommunityRole>,
    assignments: Vec<(UserId, Uuid)>,
    members: HashSet<UserId>,
    events: Vec<CommunityEvent>,
}

impl Community {
    pub fn restore(
        id: CommunityId,
        name: CommunityName,
        owner_id: UserId,
        description: Option<CommunityDescription>,
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
            roles: Vec::new(),
            assignments: Vec::new(),
            members: HashSet::new(),
            events: Vec::new(),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn load(
        id: CommunityId,
        name: CommunityName,
        owner_id: UserId,
        description: Option<CommunityDescription>,
        accent_color: Option<String>,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
        roles: Vec<CommunityRole>,
        assignments: Vec<(UserId, Uuid)>,
        members: HashSet<UserId>,
    ) -> Self {
        Self {
            id,
            name,
            owner_id,
            description,
            accent_color,
            created_at,
            updated_at,
            roles,
            assignments,
            members,
            events: Vec::new(),
        }
    }

    pub fn create(
        name: CommunityName,
        owner_id: UserId,
        description: Option<CommunityDescription>,
        accent_color: Option<String>,
    ) -> Self {
        let now = Utc::now();
        let id = CommunityId::generate();
        let event = CommunityEvent::Created(CommunityCreated::record(
            id,
            owner_id,
            name.clone(),
            description.clone(),
            now,
        ));
        let mut members = HashSet::new();
        members.insert(owner_id);

        Self {
            id,
            name,
            owner_id,
            description,
            accent_color,
            created_at: now,
            updated_at: now,
            roles: Vec::new(),
            assignments: Vec::new(),
            members,
            events: vec![event],
        }
    }

    pub fn transfer_ownership(&mut self, new_owner: UserId, now: DateTime<Utc>) -> Result<(), PolicyError> {
        if !self.is_member(&new_owner) {
            return Err(PolicyError::NotMember);
        }
        let previous = self.owner_id;
        self.owner_id = new_owner;
        self.updated_at = now;
        self.record(CommunityEvent::OwnerChanged(CommunityOwnerChanged::record(
            self.id,
            previous,
            new_owner,
            self.member_ids(),
            now,
        )));
        Ok(())
    }

    pub fn delete(&mut self, member_ids: Vec<UserId>, now: DateTime<Utc>) {
        self.record(CommunityEvent::Deleted(CommunityDeleted::record(
            self.id,
            self.owner_id,
            member_ids,
            now,
        )));
    }

    pub fn ensure_can_manage(&self, actor: &UserId) -> Result<(), PolicyError> {
        self.ensure_permission(actor, Permission::ManageCommunity)
    }

    pub fn update_details(
        &mut self,
        actor: &UserId,
        name: CommunityName,
        description: Option<CommunityDescription>,
        now: DateTime<Utc>,
    ) -> Result<(), PolicyError> {
        self.ensure_permission(actor, Permission::ManageCommunity)?;
        self.name = name.clone();
        self.description = description.clone();
        self.updated_at = now;
        self.record(CommunityEvent::Updated(CommunityUpdated::record(
            self.id,
            name,
            description,
            self.member_ids(),
            now,
        )));
        Ok(())
    }

    pub fn create_role(
        &mut self,
        actor: &UserId,
        name: String,
        color: String,
        permissions: Permissions,
        now: DateTime<Utc>,
    ) -> Result<CommunityRole, PolicyError> {
        self.ensure_role_manager(actor)?;
        if self.roles.len() >= MAX_ROLES_PER_COMMUNITY {
            return Err(PolicyError::LimitReached);
        }
        let permissions = permissions.known();
        self.ensure_grantable(actor, permissions)?;
        let taken: HashSet<i32> = self.roles.iter().map(|r| r.position).collect();
        let position = if self.is_owner(actor) {
            taken.iter().copied().max().unwrap_or(0) + 1
        } else {
            let top = self.top_position(actor).ok_or(PolicyError::HierarchyViolation)?;
            let mut candidate = top - 1;
            while taken.contains(&candidate) {
                candidate -= 1;
            }
            candidate
        };
        let role = CommunityRole {
            id: Uuid::new_v4(),
            community_id: self.id.uuid(),
            name,
            color,
            permissions,
            position,
            created_at: now,
        };
        self.roles.push(role.clone());
        self.roles_changed(now);
        Ok(role)
    }

    pub fn update_role(
        &mut self,
        actor: &UserId,
        role_id: Uuid,
        patch: &RolePatch,
        now: DateTime<Utc>,
    ) -> Result<CommunityRole, PolicyError> {
        self.ensure_role_manager(actor)?;
        let role = self.role(role_id).ok_or(PolicyError::RoleNotFound)?.clone();
        if !self.outranks_position(actor, role.position) {
            return Err(PolicyError::HierarchyViolation);
        }
        let projected = role
            .permissions
            .union(patch.add_permissions.known())
            .difference(patch.remove_permissions.known());
        self.ensure_grantable(actor, projected.difference(role.permissions))?;
        if !role.permissions.is_subset_of(projected) {
            self.ensure_outranks_holders(actor, role_id)?;
        }
        let updated = self
            .roles
            .iter_mut()
            .find(|r| r.id == role_id)
            .ok_or(PolicyError::RoleNotFound)?;
        if let Some(name) = &patch.name {
            updated.name = name.trim().to_string();
        }
        if let Some(color) = &patch.color {
            updated.color = color.clone();
        }
        updated.permissions = projected;
        let updated = updated.clone();
        self.roles_changed(now);
        Ok(updated)
    }

    pub fn delete_role(
        &mut self,
        actor: &UserId,
        role_id: Uuid,
        now: DateTime<Utc>,
    ) -> Result<(), PolicyError> {
        self.ensure_role_manager(actor)?;
        let role = self.role(role_id).ok_or(PolicyError::RoleNotFound)?;
        if !self.outranks_position(actor, role.position) {
            return Err(PolicyError::HierarchyViolation);
        }
        self.ensure_outranks_holders(actor, role_id)?;
        self.roles.retain(|r| r.id != role_id);
        self.assignments.retain(|(_, rid)| *rid != role_id);
        self.roles_changed(now);
        Ok(())
    }

    pub fn assign_role(
        &mut self,
        actor: &UserId,
        target: &UserId,
        role_id: Uuid,
        now: DateTime<Utc>,
    ) -> Result<(), PolicyError> {
        self.ensure_role_manager(actor)?;
        let role = self.role(role_id).ok_or(PolicyError::RoleNotFound)?.clone();
        if !self.is_member(target) {
            return Err(PolicyError::TargetNotMember);
        }
        if !self.is_owner(actor) {
            if !self.outranks_position(actor, role.position) {
                return Err(PolicyError::HierarchyViolation);
            }
            self.ensure_grantable(actor, role.permissions)?;
            if !self.outranks_member(actor, target) {
                return Err(PolicyError::HierarchyViolation);
            }
        }
        if !self.assignments.contains(&(*target, role_id)) {
            self.assignments.push((*target, role_id));
        }
        self.roles_changed(now);
        Ok(())
    }

    pub fn revoke_role(
        &mut self,
        actor: &UserId,
        target: &UserId,
        role_id: Uuid,
        now: DateTime<Utc>,
    ) -> Result<(), PolicyError> {
        self.ensure_role_manager(actor)?;
        let role = self.role(role_id).ok_or(PolicyError::RoleNotFound)?.clone();
        if !self.is_owner(actor)
            && (!self.outranks_position(actor, role.position) || !self.outranks_member(actor, target))
        {
            return Err(PolicyError::HierarchyViolation);
        }
        let before = self.assignments.len();
        self.assignments.retain(|(u, rid)| !(u == target && *rid == role_id));
        if self.assignments.len() == before {
            return Err(PolicyError::RoleNotFound);
        }
        self.roles_changed(now);
        Ok(())
    }

    pub fn swap_roles(
        &mut self,
        actor: &UserId,
        role_a: Uuid,
        role_b: Uuid,
        now: DateTime<Utc>,
    ) -> Result<(), PolicyError> {
        self.ensure_role_manager(actor)?;
        let a = self.role(role_a).ok_or(PolicyError::RoleNotFound)?.position;
        let b = self.role(role_b).ok_or(PolicyError::RoleNotFound)?.position;
        if !self.outranks_position(actor, a) || !self.outranks_position(actor, b) {
            return Err(PolicyError::HierarchyViolation);
        }
        for role in &mut self.roles {
            if role.id == role_a {
                role.position = b;
            } else if role.id == role_b {
                role.position = a;
            }
        }
        self.roles_changed(now);
        Ok(())
    }

    pub fn kick_member(
        &mut self,
        actor: &UserId,
        target: &UserId,
        now: DateTime<Utc>,
    ) -> Result<(), PolicyError> {
        if actor == target || self.is_owner(target) {
            return Err(PolicyError::NotAllowed);
        }
        self.ensure_permission(actor, Permission::KickMembers)?;
        if !self.is_member(target) {
            return Err(PolicyError::TargetNotMember);
        }
        if !self.outranks_member(actor, target) {
            return Err(PolicyError::HierarchyViolation);
        }
        self.remove_membership(target, now);
        Ok(())
    }

    pub fn leave(&mut self, member: &UserId, now: DateTime<Utc>) -> Result<(), PolicyError> {
        if self.is_owner(member) {
            return Err(PolicyError::NotAllowed);
        }
        if !self.is_member(member) {
            return Err(PolicyError::NotMember);
        }
        self.remove_membership(member, now);
        Ok(())
    }

    pub fn ensure_can_mute(&self, actor: &UserId, target: &UserId) -> Result<ModerationGrant, PolicyError> {
        self.ensure_member(actor)?;
        if !self.is_member(target) {
            return Err(PolicyError::TargetNotMember);
        }
        if actor == target {
            return Ok(self.grant_for(actor));
        }
        if self.is_owner(target) {
            return Err(PolicyError::NotAllowed);
        }
        self.ensure_permission(actor, Permission::MuteMembers)?;
        if !self.outranks_member(actor, target) {
            return Err(PolicyError::HierarchyViolation);
        }
        Ok(self.grant_for(actor))
    }

    pub fn ensure_can_unmute(
        &self,
        actor: &UserId,
        target: &UserId,
        muted_by: Option<&UserId>,
    ) -> Result<ModerationGrant, PolicyError> {
        self.ensure_member(actor)?;
        if self.is_owner(actor) {
            return Ok(ModerationGrant::Owner);
        }
        if actor == target {
            return Ok(ModerationGrant::SelfService);
        }
        if muted_by.is_some_and(|by| self.is_owner(by)) {
            return Err(PolicyError::NotAllowed);
        }
        self.ensure_permission(actor, Permission::MuteMembers)?;
        if !self.outranks_member(actor, target) {
            return Err(PolicyError::HierarchyViolation);
        }
        Ok(ModerationGrant::Moderator)
    }

    pub fn ensure_can_disconnect(&self, actor: &UserId, target: &UserId) -> Result<(), PolicyError> {
        self.ensure_member(actor)?;
        if !self.is_member(target) {
            return Err(PolicyError::TargetNotMember);
        }
        if actor == target || self.is_owner(target) {
            return Err(PolicyError::NotAllowed);
        }
        self.ensure_permission(actor, Permission::DisconnectMembers)?;
        if !self.outranks_member(actor, target) {
            return Err(PolicyError::HierarchyViolation);
        }
        Ok(())
    }

    pub fn ensure_can_deafen(&self, actor: &UserId, target: &UserId) -> Result<ModerationGrant, PolicyError> {
        self.ensure_member(actor)?;
        if !self.is_member(target) {
            return Err(PolicyError::TargetNotMember);
        }
        if actor == target || self.is_owner(target) {
            return Err(PolicyError::NotAllowed);
        }
        self.ensure_permission(actor, Permission::DeafenMembers)?;
        if !self.outranks_member(actor, target) {
            return Err(PolicyError::HierarchyViolation);
        }
        Ok(if self.is_owner(actor) { ModerationGrant::Owner } else { ModerationGrant::Moderator })
    }

    pub fn ensure_can_undeafen(
        &self,
        actor: &UserId,
        target: &UserId,
        deafened_by: Option<&UserId>,
    ) -> Result<ModerationGrant, PolicyError> {
        self.ensure_member(actor)?;
        if self.is_owner(actor) {
            return Ok(ModerationGrant::Owner);
        }
        if actor == target {
            return Err(PolicyError::NotAllowed);
        }
        if deafened_by.is_some_and(|by| self.is_owner(by)) {
            return Err(PolicyError::NotAllowed);
        }
        self.ensure_permission(actor, Permission::DeafenMembers)?;
        if !self.outranks_member(actor, target) {
            return Err(PolicyError::HierarchyViolation);
        }
        Ok(ModerationGrant::Moderator)
    }

    pub fn ensure_can_manage_channels(&self, actor: &UserId) -> Result<ModerationGrant, PolicyError> {
        self.ensure_permission(actor, Permission::ManageChannels)?;
        Ok(self.grant_for(actor))
    }

    fn grant_for(&self, actor: &UserId) -> ModerationGrant {
        if self.is_owner(actor) {
            ModerationGrant::Owner
        } else if self.has(actor, Permission::MuteMembers) || self.has(actor, Permission::ManageChannels) {
            ModerationGrant::Moderator
        } else {
            ModerationGrant::SelfService
        }
    }

    pub fn ensure_can_delete_message(
        &self,
        actor: &UserId,
        author: &UserId,
    ) -> Result<(), PolicyError> {
        self.ensure_member(actor)?;
        if actor == author || self.has(actor, Permission::ManageMessages) {
            Ok(())
        } else {
            Err(PolicyError::NotAllowed)
        }
    }

    pub fn ensure_can_manage_invites(&self, actor: &UserId) -> Result<(), PolicyError> {
        self.ensure_permission(actor, Permission::ManageInvites)
    }

    pub fn can_revoke_any_invite(&self, actor: &UserId) -> bool {
        self.is_member(actor) && self.has(actor, Permission::ManageInvites)
    }

    pub fn is_owner(&self, user: &UserId) -> bool {
        &self.owner_id == user
    }

    pub fn member_count(&self) -> usize {
        self.members.len()
    }

    pub fn is_member(&self, user: &UserId) -> bool {
        self.members.contains(user)
    }

    pub fn member_ids(&self) -> Vec<UserId> {
        self.members.iter().copied().collect()
    }

    pub fn role(&self, role_id: Uuid) -> Option<&CommunityRole> {
        self.roles.iter().find(|r| r.id == role_id)
    }

    pub fn roles(&self) -> &[CommunityRole] {
        &self.roles
    }

    pub fn permissions_of(&self, user: &UserId) -> Permissions {
        if self.is_owner(user) {
            return Permissions::all();
        }
        if !self.is_member(user) {
            return Permissions::empty();
        }
        self.roles_of(user).fold(Permissions::empty(), |acc, r| acc.union(r.permissions))
    }

    pub fn has(&self, user: &UserId, permission: Permission) -> bool {
        self.permissions_of(user).contains(permission)
    }

    fn roles_of<'a>(&'a self, user: &'a UserId) -> impl Iterator<Item = &'a CommunityRole> + 'a {
        self.assignments
            .iter()
            .filter(move |(u, _)| u == user)
            .filter_map(move |(_, rid)| self.role(*rid))
    }

    fn top_position(&self, user: &UserId) -> Option<i32> {
        if !self.is_member(user) {
            return None;
        }
        self.roles_of(user).map(|r| r.position).max()
    }

    fn holders(&self, role_id: Uuid) -> Vec<UserId> {
        self.assignments
            .iter()
            .filter(|(u, rid)| *rid == role_id && self.is_member(u))
            .map(|(u, _)| *u)
            .collect()
    }

    fn outranks_position(&self, actor: &UserId, position: i32) -> bool {
        if self.is_owner(actor) {
            return true;
        }
        matches!(self.top_position(actor), Some(top) if position < top)
    }

    fn outranks_member(&self, actor: &UserId, target: &UserId) -> bool {
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

    fn ensure_member(&self, actor: &UserId) -> Result<(), PolicyError> {
        if self.is_member(actor) {
            Ok(())
        } else {
            Err(PolicyError::NotMember)
        }
    }

    fn ensure_permission(&self, actor: &UserId, permission: Permission) -> Result<(), PolicyError> {
        self.ensure_member(actor)?;
        if self.has(actor, permission) {
            Ok(())
        } else {
            Err(PolicyError::NotAllowed)
        }
    }

    fn ensure_role_manager(&self, actor: &UserId) -> Result<(), PolicyError> {
        self.ensure_member(actor)?;
        if self.is_owner(actor) || self.has(actor, Permission::ManageRoles) {
            Ok(())
        } else {
            Err(PolicyError::NotAllowed)
        }
    }

    fn ensure_grantable(&self, actor: &UserId, permissions: Permissions) -> Result<(), PolicyError> {
        if !permissions.known().is_subset_of(self.permissions_of(actor)) {
            return Err(PolicyError::NotAllowed);
        }
        Ok(())
    }

    fn ensure_outranks_holders(&self, actor: &UserId, role_id: Uuid) -> Result<(), PolicyError> {
        if self.is_owner(actor) {
            return Ok(());
        }
        for holder in self.holders(role_id) {
            if self.is_owner(&holder) || !self.outranks_member(actor, &holder) {
                return Err(PolicyError::HierarchyViolation);
            }
        }
        Ok(())
    }

    fn remove_membership(&mut self, user: &UserId, now: DateTime<Utc>) {
        self.members.remove(user);
        self.assignments.retain(|(u, _)| u != user);
        self.record(CommunityEvent::MemberLeft(CommunityMemberLeft::record(
            self.id, *user, now,
        )));
    }

    fn roles_changed(&mut self, now: DateTime<Utc>) {
        self.record(CommunityEvent::RolesChanged(CommunityRolesChanged::record(
            self.id,
            self.member_ids(),
            now,
        )));
    }

    fn record(&mut self, event: CommunityEvent) {
        self.events.push(event);
    }

    pub fn pull_events(&mut self) -> Vec<CommunityEvent> {
        std::mem::take(&mut self.events)
    }

    pub fn id(&self) -> &CommunityId {
        &self.id
    }

    pub fn name(&self) -> &CommunityName {
        &self.name
    }

    pub fn owner_id(&self) -> &UserId {
        &self.owner_id
    }

    pub fn description(&self) -> Option<&CommunityDescription> {
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
