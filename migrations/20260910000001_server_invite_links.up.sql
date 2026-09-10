CREATE TABLE messaging.server_invite_links (
    token       TEXT PRIMARY KEY,
    server_id   UUID NOT NULL REFERENCES messaging.servers(id) ON DELETE CASCADE,
    created_by  UUID NOT NULL,
    name        VARCHAR(60),
    is_primary  BOOLEAN NOT NULL DEFAULT false,
    max_uses    INTEGER,
    uses        INTEGER NOT NULL DEFAULT 0,
    expires_at  TIMESTAMPTZ,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    revoked_at  TIMESTAMPTZ
);

CREATE INDEX idx_server_invite_links_server ON messaging.server_invite_links (server_id);
CREATE UNIQUE INDEX uq_server_invite_links_primary ON messaging.server_invite_links (server_id) WHERE is_primary AND revoked_at IS NULL;

DROP TABLE IF EXISTS messaging.server_invite_codes;
