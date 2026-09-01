DELETE FROM messaging.dm_messages a
USING messaging.dm_messages b
WHERE a.call_id IS NOT NULL
  AND a.call_id = b.call_id
  AND a.recipient_id = b.recipient_id
  AND a.ctid > b.ctid;

CREATE UNIQUE INDEX dm_messages_call_log_unique
    ON messaging.dm_messages (call_id, recipient_id)
    WHERE call_id IS NOT NULL;
DROP TABLE IF EXISTS messaging.call_log_consumed;
