DROP INDEX IF EXISTS messaging.messaging_user_profiles_paid_idx;

ALTER TABLE messaging.messaging_user_profiles
    DROP CONSTRAINT IF EXISTS messaging_user_profiles_tier_check,
    DROP COLUMN IF EXISTS badge_updated_at,
    DROP COLUMN IF EXISTS badge_style,
    DROP COLUMN IF EXISTS tier_updated_at,
    DROP COLUMN IF EXISTS tier_valid_until,
    DROP COLUMN IF EXISTS tier;
