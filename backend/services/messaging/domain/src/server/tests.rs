use std::collections::HashSet;

use chrono::Utc;
use uuid::Uuid;

use super::{Server, events::ServerEvent, id::ServerId, name::ServerName, policy::MAX_ROLES_PER_SERVER, policy::PolicyError, role::PERM_ALL, role::PERM_KICK_MEMBERS, role::PERM_MANAGE_ROLES, role::PERM_MUTE_MEMBERS, role::RolePatch, role::ServerRole};
use domain::user::id::UserId;
use crate::channel::moderation::ModerationGrant;

struct Fixture {
    server: Server,
    owner: UserId,
    admin: UserId,
    moderator: UserId,
    member: UserId,
    admin_role: Uuid,
    moderator_role: Uuid,
}

fn role(server_id: &ServerId, permissions: i64, position: i32) -> ServerRole {
    ServerRole {
        id: Uuid::new_v4(),
        server_id: server_id.uuid(),
        name: format!("role-{position}"),
        color: "#ffffff".to_string(),
        permissions,
        position,
        created_at: Utc::now(),
    }
}

fn fixture() -> Fixture {
    let server_id = ServerId::generate();
    let owner = UserId::generate();
    let admin = UserId::generate();
    let moderator = UserId::generate();
    let member = UserId::generate();
    let admin_role = role(&server_id, PERM_MANAGE_ROLES | PERM_KICK_MEMBERS | PERM_MUTE_MEMBERS, 10);
    let moderator_role = role(&server_id, PERM_MUTE_MEMBERS, 5);
    let admin_role_id = admin_role.id;
    let moderator_role_id = moderator_role.id;
    let members: HashSet<UserId> = [owner, admin, moderator, member].into_iter().collect();
    let server = Server::load(
        server_id,
        ServerName::new("rift".to_string()),
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
        server,
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
        f.server.create_role(&stranger, "x".into(), "#000000".into(), 0, Utc::now()).unwrap_err(),
        PolicyError::NotMember
    );
    assert_eq!(
        f.server.create_role(&f.member, "x".into(), "#000000".into(), 0, Utc::now()).unwrap_err(),
        PolicyError::NotAllowed
    );
}

#[test]
fn created_roles_sit_below_the_creator_and_cannot_exceed_their_permissions() {
    let mut f = fixture();
    let err = f
        .server
        .create_role(&f.admin, "super".into(), "#000000".into(), PERM_ALL, Utc::now())
        .unwrap_err();
    assert_eq!(err, PolicyError::NotAllowed);

    let created = f
        .server
        .create_role(&f.admin, "helper".into(), "#000000".into(), PERM_MUTE_MEMBERS, Utc::now())
        .unwrap();
    assert!(created.position < 10);
    assert!(created.position != 5);

    let owner_role = f
        .server
        .create_role(&f.owner, "top".into(), "#000000".into(), PERM_ALL, Utc::now())
        .unwrap();
    assert_eq!(owner_role.position, 11);
    assert!(matches!(f.server.pull_events().as_slice(), [ServerEvent::RolesChanged(_), ServerEvent::RolesChanged(_)]));
}

#[test]
fn role_limit_is_enforced() {
    let mut f = fixture();
    for _ in 0..(MAX_ROLES_PER_SERVER - 2) {
        f.server.create_role(&f.owner, "r".into(), "#000000".into(), 0, Utc::now()).unwrap();
    }
    assert_eq!(
        f.server.create_role(&f.owner, "r".into(), "#000000".into(), 0, Utc::now()).unwrap_err(),
        PolicyError::LimitReached
    );
}

#[test]
fn hierarchy_guards_assignment_update_and_deletion() {
    let mut f = fixture();
    assert_eq!(
        f.server.assign_role(&f.admin, &f.member, f.admin_role, Utc::now()).unwrap_err(),
        PolicyError::HierarchyViolation
    );
    f.server.assign_role(&f.admin, &f.member, f.moderator_role, Utc::now()).unwrap();
    assert!(f.server.has(&f.member, PERM_MUTE_MEMBERS));

    let patch = RolePatch { add_permissions: PERM_MANAGE_ROLES, ..Default::default() };
    assert_eq!(
        f.server.update_role(&f.moderator, f.moderator_role, &patch, Utc::now()).unwrap_err(),
        PolicyError::NotAllowed
    );
    assert_eq!(
        f.server.delete_role(&f.admin, f.admin_role, Utc::now()).unwrap_err(),
        PolicyError::HierarchyViolation
    );
    f.server.delete_role(&f.owner, f.admin_role, Utc::now()).unwrap();
    assert!(f.server.role(f.admin_role).is_none());
    assert!(!f.server.has(&f.admin, PERM_MANAGE_ROLES));
}

#[test]
fn revoking_a_missing_assignment_is_reported() {
    let mut f = fixture();
    assert_eq!(
        f.server.revoke_role(&f.owner, &f.member, f.moderator_role, Utc::now()).unwrap_err(),
        PolicyError::RoleNotFound
    );
    f.server.revoke_role(&f.owner, &f.moderator, f.moderator_role, Utc::now()).unwrap();
    assert!(!f.server.has(&f.moderator, PERM_MUTE_MEMBERS));
}

#[test]
fn kicking_respects_owner_self_and_rank() {
    let mut f = fixture();
    assert_eq!(f.server.kick_member(&f.admin, &f.owner, Utc::now()).unwrap_err(), PolicyError::NotAllowed);
    assert_eq!(f.server.kick_member(&f.admin, &f.admin, Utc::now()).unwrap_err(), PolicyError::NotAllowed);
    assert_eq!(f.server.kick_member(&f.moderator, &f.member, Utc::now()).unwrap_err(), PolicyError::NotAllowed);
    f.server.kick_member(&f.admin, &f.moderator, Utc::now()).unwrap();
    assert!(!f.server.is_member(&f.moderator));
    assert!(matches!(f.server.pull_events().as_slice(), [ServerEvent::MemberLeft(_)]));
}

#[test]
fn owner_cannot_leave_but_members_can() {
    let mut f = fixture();
    assert_eq!(f.server.leave(&f.owner, Utc::now()).unwrap_err(), PolicyError::NotAllowed);
    assert_eq!(f.server.leave(&UserId::generate(), Utc::now()).unwrap_err(), PolicyError::NotMember);
    f.server.leave(&f.member, Utc::now()).unwrap();
    assert!(!f.server.is_member(&f.member));
}

#[test]
fn deafen_requires_permission_rank_and_never_targets_self_or_owner() {
    let f = fixture();
    assert_eq!(f.server.ensure_can_deafen(&f.member, &f.member).unwrap_err(), PolicyError::NotAllowed);
    assert_eq!(f.server.ensure_can_deafen(&f.moderator, &f.member).unwrap_err(), PolicyError::NotAllowed);
    assert_eq!(f.server.ensure_can_deafen(&f.admin, &f.owner).unwrap_err(), PolicyError::NotAllowed);
    assert_eq!(f.server.ensure_can_deafen(&f.owner, &f.owner).unwrap_err(), PolicyError::NotAllowed);
    assert_eq!(f.server.ensure_can_deafen(&f.owner, &f.member).unwrap(), ModerationGrant::Owner);
    assert_eq!(
        f.server.ensure_can_undeafen(&f.member, &f.member, Some(&f.owner)).unwrap_err(),
        PolicyError::NotAllowed
    );
    assert_eq!(
        f.server.ensure_can_undeafen(&f.admin, &f.member, Some(&f.owner)).unwrap_err(),
        PolicyError::NotAllowed
    );
    assert_eq!(f.server.ensure_can_undeafen(&f.owner, &f.member, Some(&f.admin)).unwrap(), ModerationGrant::Owner);
}

#[test]
fn mute_authority_follows_rank_and_mute_authorship() {
    let f = fixture();
    assert!(f.server.ensure_can_mute(&f.member, &f.member).is_ok());
    assert_eq!(f.server.ensure_can_mute(&f.member, &f.moderator).unwrap_err(), PolicyError::NotAllowed);
    assert_eq!(f.server.ensure_can_mute(&f.moderator, &f.admin).unwrap_err(), PolicyError::HierarchyViolation);
    assert_eq!(f.server.ensure_can_mute(&f.admin, &f.owner).unwrap_err(), PolicyError::NotAllowed);
    assert!(f.server.ensure_can_mute(&f.admin, &f.moderator).is_ok());

    assert_eq!(
        f.server.ensure_can_unmute(&f.admin, &f.member, Some(&f.owner)).unwrap_err(),
        PolicyError::NotAllowed
    );
    assert!(f.server.ensure_can_unmute(&f.owner, &f.member, Some(&f.admin)).is_ok());
    assert!(f.server.ensure_can_unmute(&f.member, &f.member, Some(&f.member)).is_ok());
    assert_eq!(
        f.server.ensure_can_unmute(&f.member, &f.member, Some(&f.moderator)).unwrap_err(),
        PolicyError::NotAllowed
    );
}
