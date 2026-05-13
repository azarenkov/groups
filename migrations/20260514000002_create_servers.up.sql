CREATE TABLE servers (
    id          UUID PRIMARY KEY,
    name        VARCHAR(100) NOT NULL UNIQUE,
    owner_id    UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    description TEXT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_servers_owner_id ON servers (owner_id);
CREATE INDEX idx_servers_name     ON servers (name);

CREATE TABLE server_members (
    server_id UUID NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    user_id   UUID NOT NULL REFERENCES users(id)   ON DELETE CASCADE,
    joined_at TIMESTAMPTZ NOT NULL DEFAULT now(),

    PRIMARY KEY (server_id, user_id)
);

CREATE INDEX idx_server_members_server_id ON server_members (server_id);
CREATE INDEX idx_server_members_user_id   ON server_members (user_id);
