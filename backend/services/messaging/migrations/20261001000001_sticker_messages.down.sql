ALTER TABLE messaging.dm_messages
    DROP COLUMN IF EXISTS sticker;

ALTER TABLE messaging.messages
    DROP COLUMN IF EXISTS sticker;
