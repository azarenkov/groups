UPDATE public.outbox o
SET payload = jsonb_set(
    o.payload,
    '{recipient_id}',
    to_jsonb(
        CASE
            WHEN d.sender_id = (o.payload ->> 'sender_id')::uuid THEN d.recipient_id
            ELSE d.sender_id
        END
    )
)
FROM messaging.dm_messages d
WHERE o.aggregate_type = 'dm_message'
  AND o.event_type = 'edited'
  AND o.processed_at IS NULL
  AND NOT (o.payload ? 'recipient_id')
  AND d.id = o.aggregate_id;
