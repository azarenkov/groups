CREATE TYPE visibility AS ENUM ('public', 'private');
CREATE TYPE kind       AS ENUM ('text', 'voice');

CREATE TABLE channels (
    id            UUID PRIMARY KEY,
    server_id     UUID NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    owner_id      UUID NOT NULL REFERENCES users(id)   ON DELETE CASCADE,
    name          VARCHAR(100) NOT NULL,
    kind          kind       NOT NULL DEFAULT 'text',
    visibility    visibility NOT NULL DEFAULT 'public',
    password_hash VARCHAR(255),
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_join     TIMESTAMPTZ NOT NULL DEFAULT now(),

    UNIQUE (server_id, name),

    CONSTRAINT check_private_channel_password CHECK (
        (visibility = 'private' AND password_hash IS NOT NULL)
        OR visibility = 'public'
    )
);

CREATE INDEX idx_channels_server_id  ON channels (server_id);
CREATE INDEX idx_channels_owner_id   ON channels (owner_id);
CREATE INDEX idx_channels_name       ON channels (name);
CREATE INDEX idx_channels_visibility ON channels (visibility);
CREATE INDEX idx_channels_kind       ON channels (kind);
CREATE INDEX idx_channels_last_join  ON channels (last_join DESC);

CREATE TABLE channel_muted_participants (
    channel_id UUID NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    user_id    UUID NOT NULL REFERENCES users(id)    ON DELETE CASCADE,
    muted_by   UUID NOT NULL REFERENCES users(id)    ON DELETE RESTRICT,
    muted_at   TIMESTAMPTZ NOT NULL DEFAULT now(),

    PRIMARY KEY (channel_id, user_id)
);

CREATE INDEX idx_channel_muted_participants_user_id
    ON channel_muted_participants (user_id);

CREATE TABLE channel_voice_participants (
    channel_id UUID NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    user_id    UUID NOT NULL REFERENCES users(id)    ON DELETE CASCADE,
    session_id TEXT NOT NULL,
    joined_at  TIMESTAMPTZ NOT NULL DEFAULT now(),

    PRIMARY KEY (channel_id, user_id, session_id)
);

CREATE INDEX idx_channel_voice_participants_user_id
    ON channel_voice_participants (user_id);
