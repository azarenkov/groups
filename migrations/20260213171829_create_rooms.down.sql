DROP INDEX IF EXISTS idx_rooms_last_join;
DROP INDEX IF EXISTS idx_rooms_type;
DROP INDEX IF EXISTS idx_rooms_name;
DROP INDEX IF EXISTS idx_rooms_server_id;
DROP INDEX IF EXISTS idx_rooms_owner_id;
DROP TABLE IF EXISTS rooms CASCADE;
DROP TYPE IF EXISTS room_type CASCADE;
