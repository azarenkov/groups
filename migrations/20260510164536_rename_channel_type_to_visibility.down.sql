ALTER INDEX idx_channels_visibility RENAME TO idx_channels_type;
ALTER TABLE channels RENAME COLUMN visibility TO channel_type;
ALTER TYPE visibility RENAME TO channel_type;
