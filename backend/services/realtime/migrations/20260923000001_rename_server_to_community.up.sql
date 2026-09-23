ALTER TABLE realtime.realtime_server_members RENAME TO realtime_community_members;
ALTER TABLE realtime.realtime_deleted_servers RENAME TO realtime_deleted_communities;

ALTER TABLE realtime.realtime_community_members RENAME COLUMN server_id TO community_id;
ALTER TABLE realtime.realtime_deleted_communities RENAME COLUMN server_id TO community_id;
