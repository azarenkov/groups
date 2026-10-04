DROP INDEX IF EXISTS messaging.idx_dm_message_attachments_file_name_trgm;
DROP INDEX IF EXISTS messaging.idx_message_attachments_file_name_trgm;
DROP INDEX IF EXISTS messaging.idx_dm_messages_content_trgm;
DROP INDEX IF EXISTS messaging.idx_messages_content_trgm;
DROP EXTENSION IF EXISTS pg_trgm;
