ALTER TABLE refresh_tokens
    ADD COLUMN last_ip         TEXT,
    ADD COLUMN last_user_agent TEXT,
    ADD COLUMN last_used_at    TIMESTAMPTZ NOT NULL DEFAULT now();

UPDATE refresh_tokens SET last_used_at = created_at;

CREATE INDEX refresh_tokens_user_id_last_used_idx
    ON refresh_tokens (user_id, last_used_at DESC)
    WHERE revoked_at IS NULL;
