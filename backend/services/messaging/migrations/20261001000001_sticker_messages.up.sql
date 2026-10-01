ALTER TABLE messaging.messages
    ADD COLUMN sticker jsonb;

ALTER TABLE messaging.dm_messages
    ADD COLUMN sticker jsonb;
