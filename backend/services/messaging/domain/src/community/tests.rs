use std::collections::HashSet;

use chrono::Utc;
use uuid::Uuid;

use super::{Community, events::CommunityEvent, id::CommunityId, name::CommunityName, policy::MAX_ROLES_PER_COMMUNITY, policy::PolicyError, role::Permission, role::Permissions, role::RolePatch, role::CommunityRole};
use domain::user::id::UserId;
use crate::channel::moderation::ModerationGrant;

struct Fixture {
    community: Community,
    owner: UserId,
    admin: UserId,
    moderator: UserId,
    member: UserId,
    admin_role: Uuid,
    moderator_role: Uuid,
}

fn role(community_id: &CommunityId, permissions: Permissions, position: i32) -> CommunityRole {
    CommunityRole {
        id: Uuid::new_v4(),
        community_id: community_id.uuid(),
        name: format!("role-{position}"),
        color: "#ffffff".to_string(),
        permissions,
        position,
        created_at: Utc::now(),
    }
}

fn fixture() -> Fixture {
    let community_id = CommunityId::generate();
    let owner = UserId::generate();
    let admin = UserId::generate();
    let moderator = UserId::generate();
    let member = UserId::generate();
    let admin_role = role(
        &community_id,
        [Permission::ManageRoles, Permission::KickMembers, Permission::MuteMembers, Permission::DisconnectMembers]
            .into_iter()
            .collect(),
        10,
    );
    let moderator_role = role(&community_id, Permission::MuteMembers.into(), 5);
    let admin_role_id = admin_role.id;
    let moderator_role_id = moderator_role.id;
    let members: HashSet<UserId> = [owner, admin, moderator, member].into_iter().collect();
    let community = Community::load(
        community_id,
        CommunityName::new("rift".to_string()),
        owner,
        None,
        None,
        Utc::now(),
        Utc::now(),
        vec![admin_role, moderator_role],
        vec![(admin, admin_role_id), (moderator, moderator_role_id)],
        members,
    );
    Fixture {
        community,
        owner,
        admin,
        moderator,
        member,
        admin_role: admin_role_id,
        moderator_role: moderator_role_id,
    }
}

#[test]
fn outsiders_and_plain_members_cannot_manage_roles() {
    let mut f = fixture();
    let stranger = UserId::generate();
    assert_eq!(
        f.community.create_role(&stranger, "x".into(), "#000000".into(), Permissions::empty(), Utc::now()).unwrap_err(),
        PolicyError::NotMember
    );
    assert_eq!(
        f.community.create_role(&f.member, "x".into(), "#000000".into(), Permissions::empty(), Utc::now()).unwrap_err(),
        PolicyError::NotAllowed
    );
}

#[test]
fn created_roles_sit_below_the_creator_and_cannot_exceed_their_permissions() {
    let mut f = fixture();
    let err = f
        .community
        .create_role(&f.admin, "super".into(), "#000000".into(), Permissions::all(), Utc::now())
        .unwrap_err();
    assert_eq!(err, PolicyError::NotAllowed);

    let created = f
        .community
        .create_role(&f.admin, "helper".into(), "#000000".into(), Permission::MuteMembers.into(), Utc::now())
        .unwrap();
    assert!(created.position < 10);
    assert!(created.position != 5);

    let owner_role = f
        .community
        .create_role(&f.owner, "top".into(), "#000000".into(), Permissions::all(), Utc::now())
        .unwrap();
    assert_eq!(owner_role.position, 11);
    assert!(matches!(f.community.pull_events().as_slice(), [CommunityEvent::RolesChanged(_), CommunityEvent::RolesChanged(_)]));
}

#[test]
fn role_limit_is_enforced() {
    let mut f = fixture();
    for _ in 0..(MAX_ROLES_PER_COMMUNITY - 2) {
        f.community.create_role(&f.owner, "r".into(), "#000000".into(), Permissions::empty(), Utc::now()).unwrap();
    }
    assert_eq!(
        f.community.create_role(&f.owner, "r".into(), "#000000".into(), Permissions::empty(), Utc::now()).unwrap_err(),
        PolicyError::LimitReached
    );
}

#[test]
fn hierarchy_guards_assignment_update_and_deletion() {
    let mut f = fixture();
    assert_eq!(
        f.community.assign_role(&f.admin, &f.member, f.admin_role, Utc::now()).unwrap_err(),
        PolicyError::HierarchyViolation
    );
    f.community.assign_role(&f.admin, &f.member, f.moderator_role, Utc::now()).unwrap();
    assert!(f.community.has(&f.member, Permission::MuteMembers));

    let patch = RolePatch { add_permissions: Permission::ManageRoles.into(), ..Default::default() };
    assert_eq!(
        f.community.update_role(&f.moderator, f.moderator_role, &patch, Utc::now()).unwrap_err(),
        PolicyError::NotAllowed
    );
    assert_eq!(
        f.community.delete_role(&f.admin, f.admin_role, Utc::now()).unwrap_err(),
        PolicyError::HierarchyViolation
    );
    f.community.delete_role(&f.owner, f.admin_role, Utc::now()).unwrap();
    assert!(f.community.role(f.admin_role).is_none());
    assert!(!f.community.has(&f.admin, Permission::ManageRoles));
}

#[test]
fn revoking_a_missing_assignment_is_reported() {
    let mut f = fixture();
    assert_eq!(
        f.community.revoke_role(&f.owner, &f.member, f.moderator_role, Utc::now()).unwrap_err(),
        PolicyError::RoleNotFound
    );
    f.community.revoke_role(&f.owner, &f.moderator, f.moderator_role, Utc::now()).unwrap();
    assert!(!f.community.has(&f.moderator, Permission::MuteMembers));
}

#[test]
fn kicking_respects_owner_self_and_rank() {
    let mut f = fixture();
    assert_eq!(f.community.kick_member(&f.admin, &f.owner, Utc::now()).unwrap_err(), PolicyError::NotAllowed);
    assert_eq!(f.community.kick_member(&f.admin, &f.admin, Utc::now()).unwrap_err(), PolicyError::NotAllowed);
    assert_eq!(f.community.kick_member(&f.moderator, &f.member, Utc::now()).unwrap_err(), PolicyError::NotAllowed);
    f.community.kick_member(&f.admin, &f.moderator, Utc::now()).unwrap();
    assert!(!f.community.is_member(&f.moderator));
    assert!(matches!(f.community.pull_events().as_slice(), [CommunityEvent::MemberLeft(_)]));
}

#[test]
fn owner_cannot_leave_but_members_can() {
    let mut f = fixture();
    assert_eq!(f.community.leave(&f.owner, Utc::now()).unwrap_err(), PolicyError::NotAllowed);
    assert_eq!(f.community.leave(&UserId::generate(), Utc::now()).unwrap_err(), PolicyError::NotMember);
    f.community.leave(&f.member, Utc::now()).unwrap();
    assert!(!f.community.is_member(&f.member));
}

#[test]
fn disconnect_requires_its_own_permission_and_rank() {
    let f = fixture();
    assert!(f.community.ensure_can_disconnect(&f.admin, &f.member).is_ok());
    assert!(f.community.ensure_can_disconnect(&f.admin, &f.moderator).is_ok());
    assert!(f.community.ensure_can_disconnect(&f.owner, &f.admin).is_ok());
    assert_eq!(f.community.ensure_can_disconnect(&f.moderator, &f.member).unwrap_err(), PolicyError::NotAllowed);
    assert_eq!(f.community.ensure_can_disconnect(&f.admin, &f.owner).unwrap_err(), PolicyError::NotAllowed);
    assert_eq!(f.community.ensure_can_disconnect(&f.admin, &f.admin).unwrap_err(), PolicyError::NotAllowed);
}

#[test]
fn deafen_requires_permission_rank_and_never_targets_self_or_owner() {
    let f = fixture();
    assert_eq!(f.community.ensure_can_deafen(&f.member, &f.member).unwrap_err(), PolicyError::NotAllowed);
    assert_eq!(f.community.ensure_can_deafen(&f.moderator, &f.member).unwrap_err(), PolicyError::NotAllowed);
    assert_eq!(f.community.ensure_can_deafen(&f.admin, &f.owner).unwrap_err(), PolicyError::NotAllowed);
    assert_eq!(f.community.ensure_can_deafen(&f.owner, &f.owner).unwrap_err(), PolicyError::NotAllowed);
    assert_eq!(f.community.ensure_can_deafen(&f.owner, &f.member).unwrap(), ModerationGrant::Owner);
    assert_eq!(
        f.community.ensure_can_undeafen(&f.member, &f.member, Some(&f.owner)).unwrap_err(),
        PolicyError::NotAllowed
    );
    assert_eq!(
        f.community.ensure_can_undeafen(&f.admin, &f.member, Some(&f.owner)).unwrap_err(),
        PolicyError::NotAllowed
    );
    assert_eq!(f.community.ensure_can_undeafen(&f.owner, &f.member, Some(&f.admin)).unwrap(), ModerationGrant::Owner);
}

#[test]
fn mute_authority_follows_rank_and_mute_authorship() {
    let f = fixture();
    assert!(f.community.ensure_can_mute(&f.member, &f.member).is_ok());
    assert_eq!(f.community.ensure_can_mute(&f.member, &f.moderator).unwrap_err(), PolicyError::NotAllowed);
    assert_eq!(f.community.ensure_can_mute(&f.moderator, &f.admin).unwrap_err(), PolicyError::HierarchyViolation);
    assert_eq!(f.community.ensure_can_mute(&f.admin, &f.owner).unwrap_err(), PolicyError::NotAllowed);
    assert!(f.community.ensure_can_mute(&f.admin, &f.moderator).is_ok());

    assert_eq!(
        f.community.ensure_can_unmute(&f.admin, &f.member, Some(&f.owner)).unwrap_err(),
        PolicyError::NotAllowed
    );
    assert!(f.community.ensure_can_unmute(&f.owner, &f.member, Some(&f.admin)).is_ok());
    assert!(f.community.ensure_can_unmute(&f.member, &f.member, Some(&f.member)).is_ok());
    assert_eq!(
        f.community.ensure_can_unmute(&f.member, &f.member, Some(&f.moderator)).unwrap(),
        ModerationGrant::SelfService
    );
}

#[test]
fn a_role_with_a_permission_this_version_does_not_know_needs_the_owner_to_assign_it() {
    let community_id = CommunityId::generate();
    let owner = UserId::generate();
    let manager = UserId::generate();
    let member = UserId::generate();
    let manager_role = role(&community_id, Permissions::all(), 10);
    let future_role = role(&community_id, Permissions::from_bits_retain(1 << 20), 1);
    let manager_role_id = manager_role.id;
    let future_role_id = future_role.id;
    let mut community = Community::load(
        community_id,
        CommunityName::new("rift".to_string()),
        owner,
        None,
        None,
        Utc::now(),
        Utc::now(),
        vec![manager_role, future_role],
        vec![(manager, manager_role_id)],
        [owner, manager, member].into_iter().collect(),
    );
    assert_eq!(
        community.assign_role(&manager, &member, future_role_id, Utc::now()).unwrap_err(),
        PolicyError::NotAllowed
    );
    assert!(community.assign_role(&owner, &member, future_role_id, Utc::now()).is_ok());
}
