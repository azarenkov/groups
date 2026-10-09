ALTER TABLE identity.users
    DROP COLUMN IF EXISTS emoji_status_until,
    DROP COLUMN IF EXISTS emoji_status_sticker_id;
