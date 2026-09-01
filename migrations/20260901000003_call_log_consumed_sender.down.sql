ALTER TABLE messaging.call_log_consumed DROP CONSTRAINT call_log_consumed_pair_unique;

DELETE FROM messaging.call_log_consumed a
USING messaging.call_log_consumed b
WHERE a.call_id = b.call_id
  AND a.recipient_id = b.recipient_id
  AND a.ctid > b.ctid;

ALTER TABLE messaging.call_log_consumed
    ADD CONSTRAINT call_log_consumed_unique UNIQUE (call_id, recipient_id);
ALTER TABLE messaging.call_log_consumed DROP COLUMN IF EXISTS sender_id;
