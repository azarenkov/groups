ALTER TABLE identity.voice_settings ADD COLUMN noise_mode VARCHAR(10) NOT NULL DEFAULT 'standard';
ALTER TABLE identity.voice_settings
    ADD CONSTRAINT voice_settings_noise_mode_check CHECK (noise_mode IN ('off', 'standard', 'high'));
UPDATE identity.voice_settings SET noise_mode = CASE WHEN noise_suppression THEN 'standard' ELSE 'off' END;
ALTER TABLE identity.voice_settings DROP COLUMN noise_suppression;

ALTER TABLE identity.voice_settings ADD COLUMN ptt_key VARCHAR(32) NOT NULL DEFAULT 'KeyM';
