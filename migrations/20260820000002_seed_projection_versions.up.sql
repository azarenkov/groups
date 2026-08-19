UPDATE messaging.messaging_user_profiles p
SET updated_at = u.updated_at
FROM identity.users u
WHERE u.id = p.user_id;

UPDATE voice.voice_user_profiles p
SET updated_at = u.updated_at
FROM identity.users u
WHERE u.id = p.user_id;

UPDATE realtime.realtime_user_names p
SET updated_at = u.updated_at
FROM identity.users u
WHERE u.id = p.user_id;
