ALTER TABLE messaging.channels
    ADD COLUMN voice_session_started_at timestamptz;

UPDATE messaging.channels c
SET voice_session_started_at = p.started_at
FROM (
    SELECT channel_id, min(joined_at) AS started_at
    FROM messaging.channel_voice_participants
    GROUP BY channel_id
) p
WHERE p.channel_id = c.id;

CREATE FUNCTION messaging.voice_session_start() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
    UPDATE messaging.channels
    SET voice_session_started_at = NEW.joined_at
    WHERE id = NEW.channel_id
      AND (voice_session_started_at IS NULL OR voice_session_started_at > NEW.joined_at);
    RETURN NEW;
END;
$$;

CREATE FUNCTION messaging.voice_session_end() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
    PERFORM 1 FROM messaging.channels WHERE id = OLD.channel_id FOR UPDATE;
    UPDATE messaging.channels
    SET voice_session_started_at = NULL
    WHERE id = OLD.channel_id
      AND voice_session_started_at IS NOT NULL
      AND NOT EXISTS (
          SELECT 1 FROM messaging.channel_voice_participants p
          WHERE p.channel_id = OLD.channel_id
      );
    RETURN OLD;
END;
$$;

CREATE TRIGGER voice_session_start AFTER INSERT ON messaging.channel_voice_participants
    FOR EACH ROW EXECUTE FUNCTION messaging.voice_session_start();

CREATE CONSTRAINT TRIGGER voice_session_end AFTER DELETE ON messaging.channel_voice_participants
    DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW EXECUTE FUNCTION messaging.voice_session_end();
