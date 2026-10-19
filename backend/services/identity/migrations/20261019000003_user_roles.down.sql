ALTER TABLE identity.users
    DROP CONSTRAINT IF EXISTS users_role_check,
    DROP COLUMN IF EXISTS role;
