ALTER TABLE messaging.messaging_user_profiles
    ADD COLUMN emoji_status_sticker_id UUID,
    ADD COLUMN emoji_status_until TIMESTAMPTZ;
