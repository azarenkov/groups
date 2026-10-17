INSERT INTO messaging.messaging_user_tiers (user_id, tier, valid_until, updated_at, member_number)
SELECT user_id, 'free', NULL, to_timestamp(0), member_number
FROM messaging.messaging_user_profiles
WHERE member_number IS NOT NULL
ON CONFLICT (user_id) DO UPDATE
    SET member_number = COALESCE(messaging_user_tiers.member_number, EXCLUDED.member_number);

DROP INDEX IF EXISTS messaging.messaging_user_profiles_paid_idx;

ALTER TABLE messaging.messaging_user_profiles
    DROP CONSTRAINT IF EXISTS messaging_user_profiles_tier_check,
    DROP COLUMN IF EXISTS tier,
    DROP COLUMN IF EXISTS tier_valid_until,
    DROP COLUMN IF EXISTS tier_updated_at,
    DROP COLUMN IF EXISTS member_number;
