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

INSERT INTO messaging.channels (id, server_id, owner_id, name, kind, visibility, password_hash, created_at, updated_at, last_join, is_default)
SELECT gen_random_uuid(), s.id, s.owner_id,
       CASE WHEN EXISTS (SELECT 1 FROM messaging.channels n WHERE n.server_id = s.id AND n.name = 'general')
            THEN 'general-' || left(replace(s.id::text, '-', ''), 8)
            ELSE 'general' END,
       'text', 'public', NULL, now(), now(), now(), true
FROM messaging.servers s
WHERE NOT EXISTS (SELECT 1 FROM messaging.channels c WHERE c.server_id = s.id AND c.kind = 'text');

INSERT INTO messaging.channel_text_participants (channel_id, user_id, joined_at)
SELECT c.id, c.owner_id, c.created_at
FROM messaging.channels c
INNER JOIN messaging.server_members m ON m.server_id = c.server_id AND m.user_id = c.owner_id
WHERE c.is_default
ON CONFLICT (channel_id, user_id) DO NOTHING;

ALTER TABLE messaging.messages ADD COLUMN kind TEXT NOT NULL DEFAULT 'user';
ALTER TABLE messaging.messages ADD CONSTRAINT messages_kind_check CHECK (kind IN ('user', 'system'));
ALTER TABLE messaging.messages ADD COLUMN system_payload JSONB;
