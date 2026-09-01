ALTER TABLE messaging.dm_messages ADD COLUMN call_id UUID;

CREATE UNIQUE INDEX dm_messages_call_log_unique
    ON messaging.dm_messages (call_id, recipient_id)
    WHERE call_id IS NOT NULL;
