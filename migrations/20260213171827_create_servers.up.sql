CREATE TABLE IF NOT EXISTS servers (
    id UUID PRIMARY KEY,
    name VARCHAR(100) NOT NULL UNIQUE,
    owner_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    description TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_servers_owner_id ON servers(owner_id);
CREATE INDEX idx_servers_name ON servers(name);
