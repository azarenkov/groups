ALTER TABLE media.media_community_owners RENAME COLUMN community_id TO server_id;
ALTER TABLE media.media_community_logos RENAME COLUMN community_id TO server_id;

ALTER TABLE media.media_community_owners RENAME TO media_server_owners;
ALTER TABLE media.media_community_logos RENAME TO media_server_logos;
