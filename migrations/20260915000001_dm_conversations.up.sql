CREATE TABLE IF NOT EXISTS messaging.dm_conversations (
    user_id         UUID NOT NULL,
    peer_id         UUID NOT NULL,
    last_message_id UUID NOT NULL,
    last_created_at TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (user_id, peer_id)
);

CREATE INDEX IF NOT EXISTS idx_dm_conversations_user_cursor
    ON messaging.dm_conversations (user_id, last_created_at DESC, last_message_id DESC);

CREATE OR REPLACE FUNCTION messaging.dm_conversations_sync() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    latest RECORD;
    current_last UUID;
BEGIN
    IF TG_OP = 'INSERT' THEN
        INSERT INTO messaging.dm_conversations (user_id, peer_id, last_message_id, last_created_at)
        SELECT v.user_id, v.peer_id, NEW.id, NEW.created_at
        FROM (VALUES (NEW.sender_id, NEW.recipient_id), (NEW.recipient_id, NEW.sender_id)) AS v(user_id, peer_id)
        ORDER BY v.user_id, v.peer_id
        ON CONFLICT (user_id, peer_id) DO UPDATE
            SET last_message_id = EXCLUDED.last_message_id,
                last_created_at = EXCLUDED.last_created_at
            WHERE (messaging.dm_conversations.last_created_at, messaging.dm_conversations.last_message_id)
                < (EXCLUDED.last_created_at, EXCLUDED.last_message_id);
        RETURN NEW;
    END IF;

    PERFORM 1 FROM messaging.dm_conversations
    WHERE (user_id = OLD.sender_id AND peer_id = OLD.recipient_id)
       OR (user_id = OLD.recipient_id AND peer_id = OLD.sender_id)
    ORDER BY user_id, peer_id
    FOR UPDATE;

    SELECT last_message_id INTO current_last
    FROM messaging.dm_conversations
    WHERE user_id = OLD.sender_id AND peer_id = OLD.recipient_id;

    IF current_last IS NULL THEN
        RETURN OLD;
    END IF;
    IF current_last <> OLD.id
       AND EXISTS (SELECT 1 FROM messaging.dm_messages WHERE id = current_last) THEN
        RETURN OLD;
    END IF;

    SELECT id, created_at INTO latest
    FROM messaging.dm_messages
    WHERE (sender_id = OLD.sender_id AND recipient_id = OLD.recipient_id)
       OR (sender_id = OLD.recipient_id AND recipient_id = OLD.sender_id)
    ORDER BY created_at DESC, id DESC
    LIMIT 1;

    IF latest.id IS NULL THEN
        DELETE FROM messaging.dm_conversations
        WHERE (user_id = OLD.sender_id AND peer_id = OLD.recipient_id)
           OR (user_id = OLD.recipient_id AND peer_id = OLD.sender_id);
    ELSE
        UPDATE messaging.dm_conversations
        SET last_message_id = latest.id, last_created_at = latest.created_at
        WHERE (user_id = OLD.sender_id AND peer_id = OLD.recipient_id)
           OR (user_id = OLD.recipient_id AND peer_id = OLD.sender_id);
    END IF;
    RETURN OLD;
END;
$$;

DROP TRIGGER IF EXISTS dm_conversations_sync ON messaging.dm_messages;
CREATE TRIGGER dm_conversations_sync
    AFTER INSERT OR DELETE ON messaging.dm_messages
    FOR EACH ROW EXECUTE FUNCTION messaging.dm_conversations_sync();

INSERT INTO messaging.dm_conversations (user_id, peer_id, last_message_id, last_created_at)
SELECT DISTINCT ON (x.user_id, x.peer_id) x.user_id, x.peer_id, x.id, x.created_at
FROM (
    SELECT sender_id AS user_id, recipient_id AS peer_id, id, created_at FROM messaging.dm_messages
    UNION ALL
    SELECT recipient_id AS user_id, sender_id AS peer_id, id, created_at FROM messaging.dm_messages
) x
ORDER BY x.user_id, x.peer_id, x.created_at DESC, x.id DESC
ON CONFLICT (user_id, peer_id) DO NOTHING;
