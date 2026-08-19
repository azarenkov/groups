ALTER TABLE messaging.messaging_user_profiles
    ADD COLUMN updated_at TIMESTAMPTZ NOT NULL DEFAULT now();
ALTER TABLE voice.voice_user_profiles
    ADD COLUMN updated_at TIMESTAMPTZ NOT NULL DEFAULT now();
ALTER TABLE realtime.realtime_user_names
    ADD COLUMN updated_at TIMESTAMPTZ NOT NULL DEFAULT now();
