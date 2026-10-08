CREATE INDEX IF NOT EXISTS idx_users_email_lower ON identity.users (lower(email));
