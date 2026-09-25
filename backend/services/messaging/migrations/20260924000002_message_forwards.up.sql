ALTER TABLE messaging.messages
    ADD COLUMN forwarded_from jsonb;

ALTER TABLE messaging.dm_messages
    ADD COLUMN forwarded_from jsonb;

CREATE INDEX idx_message_attachments_storage_key ON messaging.message_attachments (storage_key);

CREATE INDEX idx_dm_message_attachments_storage_key ON messaging.dm_message_attachments (storage_key);
