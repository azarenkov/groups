ALTER TABLE identity.voice_settings ADD COLUMN noise_suppression BOOLEAN NOT NULL DEFAULT true;
UPDATE identity.voice_settings SET noise_suppression = noise_mode <> 'off';
ALTER TABLE identity.voice_settings DROP COLUMN noise_mode;
ALTER TABLE identity.voice_settings DROP COLUMN ptt_key;
