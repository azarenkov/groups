CREATE TABLE refresh_tokens (
    id         UUID PRIMARY KEY,
    user_id    UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    expires_at TIMESTAMPTZ NOT NULL,
    revoked_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),

    CONSTRAINT refresh_tokens_expires_after_created_check
        CHECK (expires_at > created_at)
);

CREATE INDEX refresh_tokens_user_id_active_idx
    ON refresh_tokens (user_id)
    WHERE revoked_at IS NULL;

CREATE INDEX refresh_tokens_expires_idx
    ON refresh_tokens (expires_at)
    WHERE revoked_at IS NULL;
