ALTER TABLE messaging.reports DROP CONSTRAINT reports_target_type_check;
UPDATE messaging.reports SET target_type = 'server' WHERE target_type = 'community';
ALTER TABLE messaging.reports ADD CONSTRAINT reports_target_type_check CHECK (target_type IN ('user', 'server'));

UPDATE messaging.outbox
SET event_type = replace(event_type, 'community.', 'server.'),
    payload = replace(
      replace(
        replace(payload::text, '"invite_community_id"', '"invite_server_id"'),
        '"invite_community_name"', '"invite_server_name"'),
      '"community_id"', '"server_id"')::jsonb
WHERE processed_at IS NULL
  AND (event_type LIKE 'community.%' OR payload::text LIKE '%"community_id"%'
       OR payload::text LIKE '%"invite_community_id"%'
       OR payload::text LIKE '%"invite_community_name"%');

ALTER TYPE messaging.aggregate_type RENAME VALUE 'community' TO 'server';

ALTER TABLE messaging.dm_messages RENAME COLUMN invite_community_id TO invite_server_id;
ALTER TABLE messaging.community_invite_links RENAME COLUMN community_id TO server_id;
ALTER TABLE messaging.community_member_roles RENAME COLUMN community_id TO server_id;
ALTER TABLE messaging.community_roles RENAME COLUMN community_id TO server_id;
ALTER TABLE messaging.community_members RENAME COLUMN community_id TO server_id;
ALTER TABLE messaging.channels RENAME COLUMN community_id TO server_id;

ALTER TABLE messaging.community_invite_links RENAME TO server_invite_links;
ALTER TABLE messaging.community_member_roles RENAME TO server_member_roles;
ALTER TABLE messaging.community_roles RENAME TO server_roles;
ALTER TABLE messaging.community_members RENAME TO server_members;
ALTER TABLE messaging.communities RENAME TO servers;
