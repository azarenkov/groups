ALTER TABLE messaging.dm_messages
    DROP COLUMN reply_to_message_id;

ALTER TABLE messaging.messages
    DROP COLUMN reply_to_message_id;
