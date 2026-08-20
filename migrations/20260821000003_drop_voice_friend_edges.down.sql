CREATE TABLE voice.voice_friend_edges (
    user_id    UUID NOT NULL,
    friend_id  UUID NOT NULL,
    removed    BOOLEAN NOT NULL DEFAULT FALSE,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, friend_id)
);

INSERT INTO voice.voice_friend_edges (user_id, friend_id, removed, updated_at)
SELECT user_id, friend_id, FALSE, updated_at
FROM messaging.friendships
WHERE status = 'accepted'
ON CONFLICT DO NOTHING;
