CREATE TYPE kind AS ENUM ('text', 'voice');

ALTER TABLE channels
    ADD COLUMN kind kind NOT NULL DEFAULT 'text';

CREATE INDEX idx_channels_kind ON channels (kind);
