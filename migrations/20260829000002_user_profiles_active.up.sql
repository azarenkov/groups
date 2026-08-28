ALTER TABLE messaging.messaging_user_profiles ADD COLUMN active BOOLEAN NOT NULL DEFAULT true;
ALTER TABLE voice.voice_user_profiles ADD COLUMN active BOOLEAN NOT NULL DEFAULT true;

UPDATE messaging.messaging_user_profiles p
SET active = false
FROM identity.users u
WHERE u.id = p.user_id AND (u.disabled_at IS NOT NULL OR u.deleted_at IS NOT NULL);

UPDATE voice.voice_user_profiles p
SET active = false
FROM identity.users u
WHERE u.id = p.user_id AND (u.disabled_at IS NOT NULL OR u.deleted_at IS NOT NULL);
