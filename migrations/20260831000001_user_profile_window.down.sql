ALTER TABLE identity.users DROP COLUMN timezone;
ALTER TABLE messaging.messaging_user_profiles DROP COLUMN timezone, DROP COLUMN registered_at;
ALTER TABLE voice.voice_user_profiles DROP COLUMN timezone, DROP COLUMN registered_at;
