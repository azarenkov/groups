ALTER TABLE media.media_server_logos RENAME TO media_community_logos;
ALTER TABLE media.media_server_owners RENAME TO media_community_owners;

ALTER TABLE media.media_community_logos RENAME COLUMN server_id TO community_id;
ALTER TABLE media.media_community_owners RENAME COLUMN server_id TO community_id;
