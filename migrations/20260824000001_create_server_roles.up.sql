CREATE TABLE messaging.server_roles (
    id          UUID PRIMARY KEY,
    server_id   UUID NOT NULL REFERENCES messaging.servers(id) ON DELETE CASCADE,
    name        TEXT NOT NULL,
    color       TEXT NOT NULL DEFAULT '#8e8e93',
    permissions BIGINT NOT NULL DEFAULT 0,
    position    INT NOT NULL DEFAULT 0,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_server_roles_server ON messaging.server_roles (server_id);

CREATE TABLE messaging.server_member_roles (
    server_id UUID NOT NULL,
    user_id   UUID NOT NULL,
    role_id   UUID NOT NULL REFERENCES messaging.server_roles(id) ON DELETE CASCADE,
    PRIMARY KEY (server_id, user_id, role_id),
    FOREIGN KEY (server_id, user_id)
        REFERENCES messaging.server_members(server_id, user_id) ON DELETE CASCADE
);

CREATE INDEX idx_server_member_roles_user ON messaging.server_member_roles (server_id, user_id);
