DELETE FROM realtime.realtime_friend_edges WHERE removed;
ALTER TABLE realtime.realtime_friend_edges
    DROP COLUMN removed,
    DROP COLUMN updated_at;

DELETE FROM realtime.realtime_server_members WHERE removed;
ALTER TABLE realtime.realtime_server_members
    DROP COLUMN removed,
    DROP COLUMN updated_at;

DROP TABLE realtime.realtime_deleted_servers;

DELETE FROM voice.voice_friend_edges WHERE removed;
ALTER TABLE voice.voice_friend_edges
    DROP COLUMN removed,
    DROP COLUMN updated_at;

DELETE FROM media.media_server_owners WHERE deleted_at IS NOT NULL;
ALTER TABLE media.media_server_owners
    DROP COLUMN updated_at,
    DROP COLUMN deleted_at;
