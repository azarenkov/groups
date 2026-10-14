DELETE FROM messaging.dm_messages m
USING messaging.user_blocks b
WHERE m.invite_community_id IS NOT NULL
  AND ((b.blocker_id = m.sender_id AND b.blocked_id = m.recipient_id)
    OR (b.blocker_id = m.recipient_id AND b.blocked_id = m.sender_id));
