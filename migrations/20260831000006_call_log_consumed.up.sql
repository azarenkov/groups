CREATE TABLE messaging.call_log_consumed (
    call_id      UUID NOT NULL,
    recipient_id UUID NOT NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),

    CONSTRAINT call_log_consumed_unique UNIQUE (call_id, recipient_id)
);

INSERT INTO messaging.call_log_consumed (call_id, recipient_id)
SELECT call_id, recipient_id FROM messaging.dm_messages WHERE call_id IS NOT NULL
ON CONFLICT DO NOTHING;

DROP INDEX IF EXISTS messaging.dm_messages_call_log_unique;
