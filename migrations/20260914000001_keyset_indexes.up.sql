CREATE INDEX IF NOT EXISTS idx_friendships_user_cursor
    ON messaging.friendships (user_id, updated_at DESC, friend_id DESC);

CREATE INDEX IF NOT EXISTS idx_friendships_user_status_cursor
    ON messaging.friendships (user_id, status, updated_at DESC, friend_id DESC);

CREATE INDEX IF NOT EXISTS idx_user_blocks_blocker_cursor
    ON messaging.user_blocks (blocker_id, created_at DESC, blocked_id DESC);

CREATE INDEX IF NOT EXISTS idx_server_invite_links_server_cursor
    ON messaging.server_invite_links (server_id, created_at DESC, token DESC)
    WHERE NOT is_primary AND revoked_at IS NULL;
