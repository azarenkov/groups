ALTER TABLE messaging.messaging_user_profiles ADD COLUMN profile_updated_at TIMESTAMPTZ;
ALTER TABLE voice.voice_user_profiles ADD COLUMN profile_updated_at TIMESTAMPTZ;
