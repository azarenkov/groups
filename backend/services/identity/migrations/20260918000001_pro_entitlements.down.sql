ALTER TABLE identity.users
    DROP CONSTRAINT IF EXISTS users_badge_style_check,
    DROP COLUMN IF EXISTS badge_style;

DROP TABLE IF EXISTS identity.user_entitlements;
