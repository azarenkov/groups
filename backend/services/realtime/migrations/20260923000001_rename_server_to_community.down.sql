ALTER TABLE realtime.realtime_deleted_communities RENAME COLUMN community_id TO server_id;
ALTER TABLE realtime.realtime_community_members RENAME COLUMN community_id TO server_id;

ALTER TABLE realtime.realtime_deleted_communities RENAME TO realtime_deleted_servers;
ALTER TABLE realtime.realtime_community_members RENAME TO realtime_server_members;
