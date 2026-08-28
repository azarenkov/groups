CREATE TABLE messaging.channel_voice_visits (
    channel_id     UUID NOT NULL REFERENCES messaging.channels(id) ON DELETE CASCADE,
    user_id        UUID NOT NULL,
    last_joined_at TIMESTAMPTZ NOT NULL DEFAULT now(),

    PRIMARY KEY (channel_id, user_id)
);

CREATE INDEX idx_channel_voice_visits_user_recent
    ON messaging.channel_voice_visits (user_id, last_joined_at DESC);

INSERT INTO messaging.channel_voice_visits (channel_id, user_id, last_joined_at)
SELECT channel_id, user_id, MAX(joined_at)
FROM messaging.channel_voice_participants
GROUP BY channel_id, user_id
ON CONFLICT DO NOTHING;
