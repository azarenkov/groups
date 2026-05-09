ALTER TABLE rooms RENAME COLUMN room_type TO channel_type;
ALTER TABLE rooms RENAME TO channels;
ALTER TYPE room_type RENAME TO channel_type;

ALTER INDEX idx_rooms_last_join RENAME TO idx_channels_last_join;
ALTER INDEX idx_rooms_name RENAME TO idx_channels_name;
ALTER INDEX idx_rooms_owner_id RENAME TO idx_channels_owner_id;
ALTER INDEX idx_rooms_server_id RENAME TO idx_channels_server_id;
ALTER INDEX idx_rooms_type RENAME TO idx_channels_type;

ALTER TABLE channels RENAME CONSTRAINT rooms_max_participants_check TO channels_max_participants_check;
ALTER TABLE channels RENAME CONSTRAINT rooms_owner_id_fkey TO channels_owner_id_fkey;
ALTER TABLE channels RENAME CONSTRAINT rooms_server_id_fkey TO channels_server_id_fkey;
ALTER TABLE channels RENAME CONSTRAINT rooms_server_id_name_key TO channels_server_id_name_key;
ALTER TABLE channels RENAME CONSTRAINT check_private_room_password TO check_private_channel_password;
