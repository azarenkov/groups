ALTER TABLE messaging.dm_message_attachments
    DROP COLUMN video_poster,
    DROP COLUMN video_size,
    DROP COLUMN video_duration_ms;

ALTER TABLE messaging.message_attachments
    DROP COLUMN video_poster,
    DROP COLUMN video_size,
    DROP COLUMN video_duration_ms;
