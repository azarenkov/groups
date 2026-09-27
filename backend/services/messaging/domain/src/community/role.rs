use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Permission {
    ManageChannels,
    ManageInvites,
    ManageMessages,
    MuteMembers,
    ManageRoles,
    ManageCommunity,
    KickMembers,
    DeafenMembers,
    DisconnectMembers,
}

impl Permission {
    pub const ALL: [Permission; 9] = [
        Permission::ManageChannels,
        Permission::ManageInvites,
        Permission::ManageMessages,
        Permission::MuteMembers,
        Permission::ManageRoles,
        Permission::ManageCommunity,
        Permission::KickMembers,
        Permission::DeafenMembers,
        Permission::DisconnectMembers,
    ];

    pub const fn bit(self) -> i64 {
        match self {
            Permission::ManageChannels => 1,
            Permission::ManageInvites => 2,
            Permission::ManageMessages => 4,
            Permission::MuteMembers => 8,
            Permission::ManageRoles => 16,
            Permission::ManageCommunity => 32,
            Permission::KickMembers => 64,
            Permission::DeafenMembers => 128,
            Permission::DisconnectMembers => 256,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Permissions(i64);

impl Permissions {
    pub const fn empty() -> Self {
        Self(0)
    }

    pub fn all() -> Self {
        Permission::ALL.into_iter().collect()
    }

    pub const fn from_bits_retain(bits: i64) -> Self {
        Self(bits)
    }

    pub const fn bits(self) -> i64 {
        self.0
    }

    pub fn known(self) -> Self {
        Self(self.0 & Self::all().0)
    }

    pub fn unknown_bits(self) -> i64 {
        self.0 & !Self::all().0
    }

    pub const fn contains(self, permission: Permission) -> bool {
        self.0 & permission.bit() != 0
    }

    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    pub const fn difference(self, other: Self) -> Self {
        Self(self.0 & !other.0)
    }

    pub const fn is_subset_of(self, other: Self) -> bool {
        self.0 & !other.0 == 0
    }

    pub fn insert(&mut self, permission: Permission) {
        self.0 |= permission.bit();
    }

    pub fn remove(&mut self, permission: Permission) {
        self.0 &= !permission.bit();
    }

    pub fn iter(self) -> impl Iterator<Item = Permission> {
        Permission::ALL.into_iter().filter(move |p| self.contains(*p))
    }
}

impl From<Permission> for Permissions {
    fn from(permission: Permission) -> Self {
        Self(permission.bit())
    }
}

impl FromIterator<Permission> for Permissions {
    fn from_iter<I: IntoIterator<Item = Permission>>(iter: I) -> Self {
        Self(iter.into_iter().fold(0, |acc, p| acc | p.bit()))
    }
}

#[derive(Debug, Clone)]
pub struct CommunityRole {
    pub id: Uuid,
    pub community_id: Uuid,
    pub name: String,
    pub color: String,
    pub permissions: Permissions,
    pub position: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Default)]
pub struct RolePatch {
    pub name: Option<String>,
    pub color: Option<String>,
    pub add_permissions: Permissions,
    pub remove_permissions: Permissions,
}

#[async_trait::async_trait]
pub trait CommunityRoleRepository {
    async fn list_roles(
        &self,
        community_id: &crate::community::id::CommunityId,
    ) -> Result<Vec<CommunityRole>, ports::repository::RepositoryError>;

    async fn member_role_ids(
        &self,
        community_id: &crate::community::id::CommunityId,
    ) -> Result<Vec<(Uuid, Uuid)>, ports::repository::RepositoryError>;

    async fn user_permissions(
        &self,
        community_id: &crate::community::id::CommunityId,
        user_id: &domain::user::id::UserId,
    ) -> Result<Permissions, ports::repository::RepositoryError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bits_stay_where_existing_roles_stored_them() {
        let expected = [1, 2, 4, 8, 16, 32, 64, 128, 256];
        let bits: Vec<i64> = Permission::ALL.iter().map(|p| p.bit()).collect();
        assert_eq!(bits, expected);
    }

    #[test]
    fn every_permission_round_trips_through_the_mask() {
        for permission in Permission::ALL {
            let set = Permissions::from(permission);
            let back = Permissions::from_bits_retain(set.bits());
            assert_eq!(back.iter().collect::<Vec<_>>(), vec![permission]);
        }
        let all = Permissions::all();
        assert_eq!(Permissions::from_bits_retain(all.bits()), all);
        assert_eq!(all.iter().count(), Permission::ALL.len());
    }

    #[test]
    fn unknown_bits_are_kept_until_dropped_explicitly() {
        let stored = Permissions::from_bits_retain(Permission::MuteMembers.bit() | 1024);
        assert_eq!(stored.bits(), 8 | 1024);
        assert_eq!(stored.unknown_bits(), 1024);
        assert_eq!(stored.known(), Permissions::from(Permission::MuteMembers));
        assert_eq!(stored.iter().collect::<Vec<_>>(), vec![Permission::MuteMembers]);
    }

    #[test]
    fn names_are_camel_case() {
        let names: Vec<String> = Permission::ALL
            .iter()
            .map(|p| serde_json::to_value(p).unwrap().as_str().unwrap().to_string())
            .collect();
        assert_eq!(
            names,
            [
                "manageChannels",
                "manageInvites",
                "manageMessages",
                "muteMembers",
                "manageRoles",
                "manageCommunity",
                "kickMembers",
                "deafenMembers",
                "disconnectMembers",
            ]
        );
        assert!(serde_json::from_str::<Permission>("\"flyAround\"").is_err());
    }
}
