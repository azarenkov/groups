CREATE TABLE realtime.realtime_user_blocks (
    blocker_id UUID NOT NULL,
    blocked_id UUID NOT NULL,
    active     BOOLEAN NOT NULL DEFAULT TRUE,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (blocker_id, blocked_id)
);
CREATE INDEX realtime_user_blocks_blocked_idx ON realtime.realtime_user_blocks (blocked_id);
