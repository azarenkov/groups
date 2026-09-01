ALTER TABLE identity.oauth_identities ADD COLUMN display_name TEXT;

CREATE TABLE identity.connection_states (
    state      UUID PRIMARY KEY,
    user_id    UUID NOT NULL,
    provider   TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
