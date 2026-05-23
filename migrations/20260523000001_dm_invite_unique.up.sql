DELETE FROM dm_messages WHERE id IN (
    SELECT id FROM (
        SELECT id, ROW_NUMBER() OVER (
            PARTITION BY sender_id, recipient_id, invite_server_id
            ORDER BY created_at ASC
        ) AS rn
        FROM dm_messages
        WHERE invite_server_id IS NOT NULL
    ) t WHERE rn > 1
);

CREATE UNIQUE INDEX dm_messages_invite_unique
    ON dm_messages (sender_id, recipient_id, invite_server_id)
    WHERE invite_server_id IS NOT NULL;
