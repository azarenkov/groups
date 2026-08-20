ALTER TABLE realtime.realtime_friend_edges
    ADD COLUMN pending BOOLEAN NOT NULL DEFAULT FALSE;

INSERT INTO realtime.realtime_friend_edges (user_id, friend_id, removed, pending, updated_at)
SELECT user_id, friend_id, FALSE, TRUE, updated_at
FROM messaging.friendships
WHERE status IN ('pending_outgoing', 'pending_incoming')
ON CONFLICT (user_id, friend_id) DO NOTHING;
