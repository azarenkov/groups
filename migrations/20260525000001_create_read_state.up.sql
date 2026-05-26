CREATE TABLE channel_read_state (
    user_id      UUID NOT NULL REFERENCES users(id)    ON DELETE CASCADE,
    channel_id   UUID NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    last_read_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, channel_id)
);

CREATE INDEX idx_channel_read_state_user ON channel_read_state (user_id);

CREATE TABLE dm_read_state (
    user_id      UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    friend_id    UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    last_read_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, friend_id)
);

CREATE INDEX idx_dm_read_state_user ON dm_read_state (user_id);
