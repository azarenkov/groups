CREATE TABLE voice_friend_edges (
    user_id   UUID NOT NULL,
    friend_id UUID NOT NULL,
    PRIMARY KEY (user_id, friend_id)
);

INSERT INTO voice_friend_edges (user_id, friend_id)
SELECT user_id, friend_id FROM friendships WHERE status = 'accepted'
ON CONFLICT DO NOTHING;
