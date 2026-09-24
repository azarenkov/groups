ALTER TABLE messaging.dm_messages
    DROP COLUMN forwarded_from;

ALTER TABLE messaging.messages
    DROP COLUMN forwarded_from;
