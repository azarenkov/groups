CREATE TABLE channel_muted_participants (
    channel_id UUID NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    user_id    UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    muted_by   UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    muted_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (channel_id, user_id)
);

CREATE INDEX idx_channel_muted_participants_user_id
    ON channel_muted_participants (user_id);
