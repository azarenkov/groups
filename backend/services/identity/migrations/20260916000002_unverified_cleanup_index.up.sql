CREATE INDEX IF NOT EXISTS users_unverified_cleanup_idx
    ON identity.users (created_at)
    WHERE email_verified_at IS NULL AND deleted_at IS NULL;
