ALTER TABLE media.attachments
    ADD COLUMN voice_duration_ms integer,
    ADD COLUMN voice_waveform bytea,
    ADD CONSTRAINT attachments_voice_check CHECK (
        (voice_duration_ms IS NULL AND voice_waveform IS NULL)
        OR (voice_duration_ms > 0 AND voice_waveform IS NOT NULL)
    );
