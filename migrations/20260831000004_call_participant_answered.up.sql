ALTER TABLE voice.call_participants ADD COLUMN answered_at TIMESTAMPTZ;

UPDATE voice.call_participants SET answered_at = updated_at WHERE status = 'joined' AND session_id IS NOT NULL;
