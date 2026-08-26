ALTER TABLE messaging.channel_muted_participants
    ADD COLUMN self_muted BOOLEAN NOT NULL DEFAULT false;
