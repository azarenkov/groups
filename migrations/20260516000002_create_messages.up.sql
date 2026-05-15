CREATE TABLE messages (
    id         UUID PRIMARY KEY,
    channel_id UUID NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    author_id  UUID NOT NULL REFERENCES users(id)    ON DELETE CASCADE,
    content    TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    edited_at  TIMESTAMPTZ
);

CREATE INDEX idx_messages_channel_created
    ON messages (channel_id, created_at DESC);

CREATE INDEX idx_messages_author
    ON messages (author_id);
