ALTER TABLE identity.voice_settings
    ADD COLUMN screen_content character varying(16),
    ADD CONSTRAINT voice_settings_screen_content_check CHECK (screen_content IS NULL OR screen_content IN ('auto', 'text', 'motion'));
