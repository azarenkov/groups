DROP TABLE IF EXISTS identity.connection_states;
ALTER TABLE identity.oauth_identities DROP COLUMN IF EXISTS display_name;
