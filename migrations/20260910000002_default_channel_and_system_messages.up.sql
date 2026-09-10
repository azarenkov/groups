ALTER TABLE messaging.channels ADD COLUMN is_default BOOLEAN NOT NULL DEFAULT false;

CREATE UNIQUE INDEX uq_channels_default_per_server ON messaging.channels (server_id) WHERE is_default;

UPDATE messaging.channels c
SET is_default = true
WHERE c.id = (
    SELECT x.id
    FROM messaging.channels x
    WHERE x.server_id = c.server_id AND x.kind = 'text'
    ORDER BY (x.name = 'general') DESC, x.created_at ASC, x.id ASC
    LIMIT 1
);

ALTER TABLE messaging.messages ADD COLUMN kind TEXT NOT NULL DEFAULT 'user';
ALTER TABLE messaging.messages ADD CONSTRAINT messages_kind_check CHECK (kind IN ('user', 'system'));
ALTER TABLE messaging.messages ADD COLUMN system_payload JSONB;
