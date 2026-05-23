DROP INDEX IF EXISTS idx_dm_messages_invite_server;
ALTER TABLE dm_messages DROP COLUMN IF EXISTS invite_server_id;
