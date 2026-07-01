CREATE TABLE channel_stream_viewers (
    channel_id  UUID NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    streamer_id UUID NOT NULL REFERENCES users(id)    ON DELETE CASCADE,
    viewer_id   UUID NOT NULL REFERENCES users(id)    ON DELETE CASCADE,
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (channel_id, streamer_id, viewer_id)
);

CREATE INDEX idx_channel_stream_viewers_lookup
    ON channel_stream_viewers (channel_id, streamer_id);
