CREATE TABLE media_user_avatars (
    user_id     UUID PRIMARY KEY,
    storage_key TEXT NOT NULL
);

CREATE TABLE media_server_logos (
    server_id   UUID PRIMARY KEY,
    storage_key TEXT NOT NULL
);

CREATE TABLE media_server_owners (
    server_id UUID PRIMARY KEY,
    owner_id  UUID NOT NULL
);

INSERT INTO media_user_avatars (user_id, storage_key)
SELECT id, avatar_key FROM users WHERE avatar_key IS NOT NULL;

INSERT INTO media_server_logos (server_id, storage_key)
SELECT id, logo_key FROM servers WHERE logo_key IS NOT NULL;

INSERT INTO media_server_owners (server_id, owner_id)
SELECT id, owner_id FROM servers;

ALTER TABLE users DROP COLUMN avatar_key;
ALTER TABLE servers DROP COLUMN logo_key;
