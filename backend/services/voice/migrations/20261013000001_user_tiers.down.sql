ALTER TABLE voice.voice_user_profiles
    DROP CONSTRAINT IF EXISTS voice_user_profiles_tier_check,
    DROP COLUMN IF EXISTS tier_updated_at,
    DROP COLUMN IF EXISTS tier_valid_until,
    DROP COLUMN IF EXISTS tier;
