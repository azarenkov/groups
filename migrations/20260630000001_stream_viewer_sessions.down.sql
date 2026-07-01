ALTER TABLE channel_stream_viewers
    DROP CONSTRAINT channel_stream_viewers_pkey;

ALTER TABLE channel_stream_viewers
    DROP COLUMN viewer_session_id;

ALTER TABLE channel_stream_viewers
    ADD PRIMARY KEY (channel_id, streamer_id, viewer_id);
