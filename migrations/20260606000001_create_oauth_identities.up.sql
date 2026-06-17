CREATE TABLE oauth_identities (
    provider   TEXT NOT NULL,
    subject    TEXT NOT NULL,
    user_id    UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    email      TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),

    PRIMARY KEY (provider, subject)
);

CREATE UNIQUE INDEX uq_oauth_identities_provider_user
    ON oauth_identities (provider, user_id);
