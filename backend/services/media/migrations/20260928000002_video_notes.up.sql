ALTER TABLE media.attachments
    ADD COLUMN video_duration_ms integer,
    ADD COLUMN video_size integer,
    ADD COLUMN video_poster bytea,
    ADD CONSTRAINT attachments_video_note_check CHECK (
        (video_duration_ms IS NULL AND video_size IS NULL AND video_poster IS NULL)
        OR (video_duration_ms > 0 AND video_size > 0 AND video_poster IS NOT NULL)
    ),
    ADD CONSTRAINT attachments_single_rich_media_check CHECK (
        voice_duration_ms IS NULL OR video_duration_ms IS NULL
    );
