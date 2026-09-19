ALTER TABLE identity.voice_settings
    DROP CONSTRAINT IF EXISTS voice_settings_stream_quality_check,
    DROP COLUMN IF EXISTS stream_quality;

ALTER TABLE identity.users
    DROP CONSTRAINT IF EXISTS users_badge_style_check,
    DROP COLUMN IF EXISTS badge_style;

DROP TABLE IF EXISTS identity.user_entitlements;
