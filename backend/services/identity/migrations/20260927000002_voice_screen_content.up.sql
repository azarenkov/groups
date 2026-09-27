ALTER TABLE identity.voice_settings
    ADD COLUMN screen_content character varying(16) DEFAULT 'auto' NOT NULL,
    ADD CONSTRAINT voice_settings_screen_content_check CHECK (screen_content IN ('auto', 'text', 'motion'));
