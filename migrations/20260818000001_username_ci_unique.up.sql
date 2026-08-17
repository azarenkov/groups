DROP INDEX IF EXISTS idx_users_name;
CREATE UNIQUE INDEX users_name_lower_key ON users (LOWER(name));
