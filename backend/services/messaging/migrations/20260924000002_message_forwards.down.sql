DROP INDEX messaging.idx_dm_message_attachments_storage_key;

DROP INDEX messaging.idx_message_attachments_storage_key;

ALTER TABLE messaging.dm_messages
    DROP COLUMN forwarded_from;

ALTER TABLE messaging.messages
    DROP COLUMN forwarded_from;
