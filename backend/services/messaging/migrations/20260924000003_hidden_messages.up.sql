CREATE TABLE messaging.hidden_messages (
    user_id uuid NOT NULL,
    message_id uuid NOT NULL,
    hidden_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, message_id)
);

CREATE INDEX idx_hidden_messages_message_id ON messaging.hidden_messages (message_id);

CREATE FUNCTION messaging.hidden_messages_cleanup() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
    DELETE FROM messaging.hidden_messages WHERE message_id = OLD.id;
    RETURN OLD;
END;
$$;

CREATE TRIGGER hidden_messages_cleanup AFTER DELETE ON messaging.messages
    FOR EACH ROW EXECUTE FUNCTION messaging.hidden_messages_cleanup();

CREATE TRIGGER hidden_messages_cleanup AFTER DELETE ON messaging.dm_messages
    FOR EACH ROW EXECUTE FUNCTION messaging.hidden_messages_cleanup();
