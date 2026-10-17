ALTER TABLE voice.voice_user_profiles
    DROP COLUMN IF EXISTS bio,
    DROP COLUMN IF EXISTS show_activity,
    DROP COLUMN IF EXISTS timezone,
    DROP COLUMN IF EXISTS registered_at,
    DROP COLUMN IF EXISTS profile_updated_at;
