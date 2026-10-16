CREATE INDEX idx_call_participants_reconnecting ON voice.call_participants (updated_at)
    WHERE status = 'joined' AND session_id IS NULL;
