ALTER TABLE identity.notification_settings
    ADD COLUMN voice_cues_enabled boolean DEFAULT true NOT NULL;
