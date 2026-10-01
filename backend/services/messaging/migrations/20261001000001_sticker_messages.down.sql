ALTER TABLE messaging.reports
    DROP COLUMN IF EXISTS context_sticker;

ALTER TABLE messaging.dm_messages
    DROP COLUMN IF EXISTS sticker;

ALTER TABLE messaging.messages
    DROP COLUMN IF EXISTS sticker;
