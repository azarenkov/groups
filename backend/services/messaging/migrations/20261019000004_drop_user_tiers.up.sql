DROP TABLE IF EXISTS messaging.messaging_user_tiers;

ALTER TABLE messaging.messaging_user_profiles
    DROP COLUMN IF EXISTS badge_style,
    DROP COLUMN IF EXISTS badge_updated_at;
