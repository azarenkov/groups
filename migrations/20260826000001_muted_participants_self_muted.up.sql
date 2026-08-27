ALTER TABLE messaging.channel_muted_participants
    ADD COLUMN self_muted BOOLEAN NOT NULL DEFAULT false;

UPDATE messaging.channel_muted_participants
SET self_muted = true
WHERE muted_by = user_id;
