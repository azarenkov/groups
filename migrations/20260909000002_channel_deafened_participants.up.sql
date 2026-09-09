CREATE TABLE messaging.channel_deafened_participants (
    channel_id  UUID NOT NULL REFERENCES messaging.channels(id) ON DELETE CASCADE,
    user_id     UUID NOT NULL,
    deafened_by UUID NOT NULL,
    deafened_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (channel_id, user_id)
);
CREATE INDEX idx_channel_deafened_participants_user_id ON messaging.channel_deafened_participants (user_id);
