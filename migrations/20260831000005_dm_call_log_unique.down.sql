DROP INDEX IF EXISTS messaging.dm_messages_call_log_unique;
ALTER TABLE messaging.dm_messages DROP COLUMN IF EXISTS call_id;
