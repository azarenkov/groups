ALTER TABLE messaging.message_attachments
    ADD COLUMN voice_duration_ms integer,
    ADD COLUMN voice_waveform bytea;

ALTER TABLE messaging.dm_message_attachments
    ADD COLUMN voice_duration_ms integer,
    ADD COLUMN voice_waveform bytea;

CREATE TABLE messaging.voice_listens (
    message_id uuid NOT NULL,
    user_id uuid NOT NULL,
    listened_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT voice_listens_pkey PRIMARY KEY (message_id, user_id)
);

CREATE FUNCTION messaging.voice_listens_cleanup() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
    DELETE FROM messaging.voice_listens WHERE message_id = OLD.id;
    RETURN OLD;
END;
$$;

CREATE TRIGGER voice_listens_cleanup AFTER DELETE ON messaging.messages
    FOR EACH ROW EXECUTE FUNCTION messaging.voice_listens_cleanup();

CREATE TRIGGER voice_listens_cleanup AFTER DELETE ON messaging.dm_messages
    FOR EACH ROW EXECUTE FUNCTION messaging.voice_listens_cleanup();
