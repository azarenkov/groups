ALTER TABLE identity.users
    DROP COLUMN pending_email,
    DROP COLUMN password_changed_at,
    DROP COLUMN disabled_at,
    DROP COLUMN deleted_at;
