ALTER TABLE voice.call_participants ADD COLUMN joined_at TIMESTAMPTZ;

UPDATE voice.call_participants SET joined_at = COALESCE(answered_at, updated_at) WHERE status = 'joined';
