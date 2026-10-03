ALTER TABLE media.media_sticker_packs
    ADD COLUMN cover_sticker_id uuid REFERENCES media.media_stickers(id) ON DELETE SET NULL;
