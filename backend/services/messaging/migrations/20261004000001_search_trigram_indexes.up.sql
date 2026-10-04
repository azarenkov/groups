CREATE EXTENSION IF NOT EXISTS pg_trgm WITH SCHEMA messaging;

DO $$
DECLARE
    trgm_schema text;
BEGIN
    SELECT n.nspname INTO STRICT trgm_schema
    FROM pg_extension e
    JOIN pg_namespace n ON n.oid = e.extnamespace
    WHERE e.extname = 'pg_trgm';

    EXECUTE format('CREATE INDEX idx_messages_content_trgm ON messaging.messages USING gin (lower(content) %I.gin_trgm_ops)', trgm_schema);
    EXECUTE format('CREATE INDEX idx_dm_messages_content_trgm ON messaging.dm_messages USING gin (lower(content) %I.gin_trgm_ops)', trgm_schema);
    EXECUTE format('CREATE INDEX idx_message_attachments_file_name_trgm ON messaging.message_attachments USING gin (lower(file_name) %I.gin_trgm_ops)', trgm_schema);
    EXECUTE format('CREATE INDEX idx_dm_message_attachments_file_name_trgm ON messaging.dm_message_attachments USING gin (lower(file_name) %I.gin_trgm_ops)', trgm_schema);
END
$$;
