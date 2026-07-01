ALTER TABLE channel_stream_viewers
    DROP CONSTRAINT channel_stream_viewers_pkey;

ALTER TABLE channel_stream_viewers
    ADD COLUMN viewer_session_id TEXT NOT NULL DEFAULT '';

ALTER TABLE channel_stream_viewers
    ALTER COLUMN viewer_session_id DROP DEFAULT;

ALTER TABLE channel_stream_viewers
    ADD PRIMARY KEY (channel_id, streamer_id, viewer_id, viewer_session_id);
