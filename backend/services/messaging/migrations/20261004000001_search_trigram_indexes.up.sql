CREATE EXTENSION IF NOT EXISTS pg_trgm WITH SCHEMA messaging;

CREATE INDEX idx_messages_content_trgm ON messaging.messages USING gin (lower(content) messaging.gin_trgm_ops);

CREATE INDEX idx_dm_messages_content_trgm ON messaging.dm_messages USING gin (lower(content) messaging.gin_trgm_ops);

CREATE INDEX idx_message_attachments_file_name_trgm ON messaging.message_attachments USING gin (lower(file_name) messaging.gin_trgm_ops);

CREATE INDEX idx_dm_message_attachments_file_name_trgm ON messaging.dm_message_attachments USING gin (lower(file_name) messaging.gin_trgm_ops);
