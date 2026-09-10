CREATE TABLE realtime_dm_pairs (
    user_a UUID NOT NULL,
    user_b UUID NOT NULL,
    PRIMARY KEY (user_a, user_b)
);

INSERT INTO realtime_dm_pairs (user_a, user_b)
SELECT DISTINCT LEAST(sender_id, recipient_id), GREATEST(sender_id, recipient_id)
FROM messaging.dm_messages
ON CONFLICT DO NOTHING;
