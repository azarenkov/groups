ALTER TABLE identity.users ADD COLUMN timezone VARCHAR(64) NOT NULL DEFAULT '';
ALTER TABLE messaging.messaging_user_profiles
    ADD COLUMN timezone VARCHAR(64) NOT NULL DEFAULT '',
    ADD COLUMN registered_at TIMESTAMPTZ;
ALTER TABLE voice.voice_user_profiles
    ADD COLUMN timezone VARCHAR(64) NOT NULL DEFAULT '',
    ADD COLUMN registered_at TIMESTAMPTZ;

UPDATE messaging.messaging_user_profiles p SET registered_at = u.created_at
FROM identity.users u WHERE u.id = p.user_id;
UPDATE voice.voice_user_profiles p SET registered_at = u.created_at
FROM identity.users u WHERE u.id = p.user_id;
