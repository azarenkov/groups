ALTER TYPE channel_type RENAME TO visibility;
ALTER TABLE channels RENAME COLUMN channel_type TO visibility;
ALTER INDEX idx_channels_type RENAME TO idx_channels_visibility;
