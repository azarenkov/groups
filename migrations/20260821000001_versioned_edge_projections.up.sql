ALTER TABLE realtime.realtime_friend_edges
    ADD COLUMN removed BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN updated_at TIMESTAMPTZ NOT NULL DEFAULT '-infinity';

UPDATE realtime.realtime_friend_edges e
SET updated_at = f.updated_at
FROM messaging.friendships f
WHERE f.user_id = e.user_id AND f.friend_id = e.friend_id;

ALTER TABLE realtime.realtime_friend_edges ALTER COLUMN updated_at SET DEFAULT now();

ALTER TABLE realtime.realtime_server_members
    ADD COLUMN removed BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN updated_at TIMESTAMPTZ NOT NULL DEFAULT '-infinity';

UPDATE realtime.realtime_server_members m
SET updated_at = s.joined_at
FROM messaging.server_members s
WHERE s.server_id = m.server_id AND s.user_id = m.user_id;

ALTER TABLE realtime.realtime_server_members ALTER COLUMN updated_at SET DEFAULT now();

CREATE TABLE realtime.realtime_deleted_servers (
    server_id  UUID PRIMARY KEY,
    deleted_at TIMESTAMPTZ NOT NULL
);

ALTER TABLE voice.voice_friend_edges
    ADD COLUMN removed BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN updated_at TIMESTAMPTZ NOT NULL DEFAULT '-infinity';

UPDATE voice.voice_friend_edges e
SET updated_at = f.updated_at
FROM messaging.friendships f
WHERE f.user_id = e.user_id AND f.friend_id = e.friend_id;

ALTER TABLE voice.voice_friend_edges ALTER COLUMN updated_at SET DEFAULT now();

ALTER TABLE media.media_server_owners
    ADD COLUMN updated_at TIMESTAMPTZ NOT NULL DEFAULT '-infinity',
    ADD COLUMN deleted_at TIMESTAMPTZ;

UPDATE media.media_server_owners o
SET updated_at = s.updated_at
FROM messaging.servers s
WHERE s.id = o.server_id;

ALTER TABLE media.media_server_owners ALTER COLUMN updated_at SET DEFAULT now();
