ALTER TABLE messaging.messages
    ADD COLUMN reply_to_message_id uuid;

ALTER TABLE messaging.dm_messages
    ADD COLUMN reply_to_message_id uuid;
