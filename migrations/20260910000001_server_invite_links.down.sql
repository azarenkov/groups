CREATE TABLE messaging.server_invite_codes (
    code       TEXT PRIMARY KEY,
    server_id  UUID NOT NULL REFERENCES messaging.servers(id) ON DELETE CASCADE,
    created_by UUID NOT NULL,
    max_uses   INTEGER,
    uses       INTEGER NOT NULL DEFAULT 0,
    expires_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    revoked_at TIMESTAMPTZ
);

CREATE INDEX idx_server_invite_codes_server ON messaging.server_invite_codes (server_id);

DROP TABLE messaging.server_invite_links;
