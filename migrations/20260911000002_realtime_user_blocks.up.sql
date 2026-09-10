CREATE TABLE realtime_user_blocks (
    blocker_id UUID NOT NULL,
    blocked_id UUID NOT NULL,
    active     BOOLEAN NOT NULL DEFAULT TRUE,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (blocker_id, blocked_id)
);
CREATE INDEX realtime_user_blocks_blocked_idx ON realtime_user_blocks (blocked_id);

INSERT INTO realtime_user_blocks (blocker_id, blocked_id, active, updated_at)
SELECT blocker_id, blocked_id, TRUE, created_at FROM messaging.user_blocks
ON CONFLICT DO NOTHING;
