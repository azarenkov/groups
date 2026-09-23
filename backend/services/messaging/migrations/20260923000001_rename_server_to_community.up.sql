ALTER TABLE messaging.servers RENAME TO communities;
ALTER TABLE messaging.server_members RENAME TO community_members;
ALTER TABLE messaging.server_roles RENAME TO community_roles;
ALTER TABLE messaging.server_member_roles RENAME TO community_member_roles;
ALTER TABLE messaging.server_invite_links RENAME TO community_invite_links;

ALTER TABLE messaging.channels RENAME COLUMN server_id TO community_id;
ALTER TABLE messaging.community_members RENAME COLUMN server_id TO community_id;
ALTER TABLE messaging.community_roles RENAME COLUMN server_id TO community_id;
ALTER TABLE messaging.community_member_roles RENAME COLUMN server_id TO community_id;
ALTER TABLE messaging.community_invite_links RENAME COLUMN server_id TO community_id;
ALTER TABLE messaging.dm_messages RENAME COLUMN invite_server_id TO invite_community_id;

ALTER TYPE messaging.aggregate_type RENAME VALUE 'server' TO 'community';

UPDATE messaging.reports SET target_type = 'community' WHERE target_type = 'server';
ALTER TABLE messaging.reports DROP CONSTRAINT reports_target_type_check;
ALTER TABLE messaging.reports ADD CONSTRAINT reports_target_type_check CHECK (target_type IN ('user', 'community'));
