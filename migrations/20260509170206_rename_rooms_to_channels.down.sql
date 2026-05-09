ALTER TABLE channels RENAME CONSTRAINT check_private_channel_password TO check_private_room_password;
ALTER TABLE channels RENAME CONSTRAINT channels_server_id_name_key TO rooms_server_id_name_key;
ALTER TABLE channels RENAME CONSTRAINT channels_server_id_fkey TO rooms_server_id_fkey;
ALTER TABLE channels RENAME CONSTRAINT channels_owner_id_fkey TO rooms_owner_id_fkey;
ALTER TABLE channels RENAME CONSTRAINT channels_max_participants_check TO rooms_max_participants_check;

ALTER INDEX idx_channels_type RENAME TO idx_rooms_type;
ALTER INDEX idx_channels_server_id RENAME TO idx_rooms_server_id;
ALTER INDEX idx_channels_owner_id RENAME TO idx_rooms_owner_id;
ALTER INDEX idx_channels_name RENAME TO idx_rooms_name;
ALTER INDEX idx_channels_last_join RENAME TO idx_rooms_last_join;

ALTER TYPE channel_type RENAME TO room_type;
ALTER TABLE channels RENAME TO rooms;
ALTER TABLE rooms RENAME COLUMN channel_type TO room_type;
