ALTER TABLE messaging.messages
    ADD COLUMN forwarded_from jsonb;

ALTER TABLE messaging.dm_messages
    ADD COLUMN forwarded_from jsonb;
