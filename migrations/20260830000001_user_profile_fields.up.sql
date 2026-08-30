ALTER TABLE identity.users
    ADD COLUMN bio VARCHAR(190) NOT NULL DEFAULT '',
    ADD COLUMN show_activity BOOLEAN NOT NULL DEFAULT true;

ALTER TABLE messaging.messaging_user_profiles
    ADD COLUMN bio VARCHAR(190) NOT NULL DEFAULT '',
    ADD COLUMN show_activity BOOLEAN NOT NULL DEFAULT true;

ALTER TABLE voice.voice_user_profiles
    ADD COLUMN bio VARCHAR(190) NOT NULL DEFAULT '',
    ADD COLUMN show_activity BOOLEAN NOT NULL DEFAULT true;
