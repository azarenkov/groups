ALTER TABLE messaging.call_log_consumed DROP CONSTRAINT call_log_consumed_pair_unique;
ALTER TABLE messaging.call_log_consumed
    ADD CONSTRAINT call_log_consumed_unique UNIQUE (call_id, recipient_id);
ALTER TABLE messaging.call_log_consumed DROP COLUMN IF EXISTS sender_id;
