ALTER TABLE identity.users
    ADD COLUMN emoji_status_sticker_id UUID,
    ADD COLUMN emoji_status_until TIMESTAMPTZ;
