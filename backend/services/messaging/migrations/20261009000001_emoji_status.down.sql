ALTER TABLE messaging.messaging_user_profiles
    DROP COLUMN IF EXISTS emoji_status_updated_at,
    DROP COLUMN IF EXISTS emoji_status_until,
    DROP COLUMN IF EXISTS emoji_status_sticker_id;
