DROP INDEX IF EXISTS users_name_lower_key;
CREATE INDEX idx_users_name ON users (name);
