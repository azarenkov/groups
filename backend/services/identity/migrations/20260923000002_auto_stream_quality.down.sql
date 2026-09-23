UPDATE identity.voice_settings SET stream_quality = '720p60' WHERE stream_quality = 'auto';

ALTER TABLE identity.voice_settings
    DROP CONSTRAINT voice_settings_stream_quality_check,
    ADD CONSTRAINT voice_settings_stream_quality_check CHECK (stream_quality IN ('720p60', '1080p60', '1440p60', '2160p60')),
    ALTER COLUMN stream_quality SET DEFAULT '720p60';
