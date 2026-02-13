CREATE TYPE room_type AS ENUM ('public', 'private');

CREATE TABLE IF NOT EXISTS rooms (
    id UUID PRIMARY KEY,
    server_id UUID NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    owner_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name VARCHAR(100) NOT NULL,
    room_type room_type NOT NULL DEFAULT 'public',
    password_hash VARCHAR(255),
    max_participants INTEGER NOT NULL DEFAULT 10 CHECK (max_participants > 0 AND max_participants <= 100),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_join TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE(server_id, name)
);

CREATE INDEX idx_rooms_server_id ON rooms(server_id);
CREATE INDEX idx_rooms_owner_id ON rooms(owner_id);
CREATE INDEX idx_rooms_name ON rooms(name);
CREATE INDEX idx_rooms_type ON rooms(room_type);
CREATE INDEX idx_rooms_last_join ON rooms(last_join DESC);

ALTER TABLE rooms ADD CONSTRAINT check_private_room_password
    CHECK (
        (room_type = 'private' AND password_hash IS NOT NULL) OR
        (room_type = 'public')
    );
