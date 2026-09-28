ALTER TABLE media.attachments
    DROP CONSTRAINT attachments_single_rich_media_check,
    DROP CONSTRAINT attachments_video_note_check,
    DROP COLUMN video_poster,
    DROP COLUMN video_size,
    DROP COLUMN video_duration_ms;
