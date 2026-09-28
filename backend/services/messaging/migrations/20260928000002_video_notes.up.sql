ALTER TABLE messaging.message_attachments
    ADD COLUMN video_duration_ms integer,
    ADD COLUMN video_size integer,
    ADD COLUMN video_poster bytea;

ALTER TABLE messaging.dm_message_attachments
    ADD COLUMN video_duration_ms integer,
    ADD COLUMN video_size integer,
    ADD COLUMN video_poster bytea;
