DROP TRIGGER voice_listens_cleanup ON messaging.dm_messages;
DROP TRIGGER voice_listens_cleanup ON messaging.messages;
DROP FUNCTION messaging.voice_listens_cleanup();

DROP TABLE messaging.voice_listens;

ALTER TABLE messaging.dm_message_attachments
    DROP COLUMN voice_waveform,
    DROP COLUMN voice_duration_ms;

ALTER TABLE messaging.message_attachments
    DROP COLUMN voice_waveform,
    DROP COLUMN voice_duration_ms;
