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
