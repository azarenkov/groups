ALTER TABLE media.attachments
    DROP CONSTRAINT attachments_voice_check,
    DROP COLUMN voice_waveform,
    DROP COLUMN voice_duration_ms;
