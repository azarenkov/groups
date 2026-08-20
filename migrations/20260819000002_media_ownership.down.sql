ALTER TABLE users ADD COLUMN avatar_key TEXT;
ALTER TABLE servers ADD COLUMN logo_key TEXT;

UPDATE users u SET avatar_key = m.storage_key
FROM media_user_avatars m WHERE m.user_id = u.id;

UPDATE servers s SET logo_key = m.storage_key
FROM media_server_logos m WHERE m.server_id = s.id;

DROP TABLE media_user_avatars;
DROP TABLE media_server_logos;
DROP TABLE media_server_owners;
