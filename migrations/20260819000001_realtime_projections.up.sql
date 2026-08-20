CREATE TABLE realtime_friend_edges (
    user_id   UUID NOT NULL,
    friend_id UUID NOT NULL,
    PRIMARY KEY (user_id, friend_id)
);

CREATE TABLE realtime_server_members (
    server_id UUID NOT NULL,
    user_id   UUID NOT NULL,
    PRIMARY KEY (server_id, user_id)
);
CREATE INDEX realtime_server_members_user_idx ON realtime_server_members (user_id);

CREATE TABLE realtime_user_names (
    user_id UUID PRIMARY KEY,
    name    TEXT NOT NULL
);

INSERT INTO realtime_friend_edges (user_id, friend_id)
SELECT user_id, friend_id FROM friendships WHERE status = 'accepted'
ON CONFLICT DO NOTHING;

INSERT INTO realtime_server_members (server_id, user_id)
SELECT server_id, user_id FROM server_members
ON CONFLICT DO NOTHING;

INSERT INTO realtime_user_names (user_id, name)
SELECT id, name FROM users
ON CONFLICT DO NOTHING;
