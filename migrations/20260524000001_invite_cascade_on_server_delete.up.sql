ALTER TABLE dm_messages
    DROP CONSTRAINT dm_messages_invite_server_id_fkey;

ALTER TABLE dm_messages
    ADD CONSTRAINT dm_messages_invite_server_id_fkey
    FOREIGN KEY (invite_server_id) REFERENCES servers(id) ON DELETE CASCADE;
