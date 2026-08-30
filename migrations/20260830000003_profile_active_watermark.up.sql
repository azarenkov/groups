ALTER TABLE messaging.messaging_user_profiles ADD COLUMN active_updated_at TIMESTAMPTZ NOT NULL DEFAULT to_timestamp(0);
ALTER TABLE voice.voice_user_profiles ADD COLUMN active_updated_at TIMESTAMPTZ NOT NULL DEFAULT to_timestamp(0);
UPDATE messaging.messaging_user_profiles SET active_updated_at = updated_at;
UPDATE voice.voice_user_profiles SET active_updated_at = updated_at;
