ALTER TABLE dm_messages
    ADD COLUMN invite_server_id UUID REFERENCES servers(id) ON DELETE SET NULL;

CREATE INDEX idx_dm_messages_invite_server
    ON dm_messages (invite_server_id)
    WHERE invite_server_id IS NOT NULL;
