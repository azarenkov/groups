ALTER TABLE messaging.call_log_consumed ADD COLUMN sender_id UUID;

UPDATE messaging.call_log_consumed c
SET sender_id = m.sender_id
FROM messaging.dm_messages m
WHERE m.call_id = c.call_id AND m.recipient_id = c.recipient_id;

ALTER TABLE messaging.call_log_consumed DROP CONSTRAINT call_log_consumed_unique;
ALTER TABLE messaging.call_log_consumed
    ADD CONSTRAINT call_log_consumed_pair_unique UNIQUE (call_id, sender_id, recipient_id);
