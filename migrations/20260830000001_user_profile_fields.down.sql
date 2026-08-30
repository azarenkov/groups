ALTER TABLE identity.users DROP COLUMN bio, DROP COLUMN show_activity;
ALTER TABLE messaging.messaging_user_profiles DROP COLUMN bio, DROP COLUMN show_activity;
ALTER TABLE voice.voice_user_profiles DROP COLUMN bio, DROP COLUMN show_activity;
