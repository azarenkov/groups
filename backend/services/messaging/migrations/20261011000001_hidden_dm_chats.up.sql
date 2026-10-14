CREATE TABLE messaging.hidden_dm_chats (
    user_id uuid NOT NULL,
    peer_id uuid NOT NULL,
    hidden_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT hidden_dm_chats_pkey PRIMARY KEY (user_id, peer_id)
);

CREATE FUNCTION messaging.hidden_dm_chats_deleted_user_cleanup() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
    DELETE FROM messaging.hidden_dm_chats
    WHERE user_id = NEW.user_id OR peer_id = NEW.user_id;
    RETURN NEW;
END;
$$;

CREATE TRIGGER hidden_dm_chats_deleted_user_cleanup
    AFTER INSERT ON messaging.messaging_deleted_users
    FOR EACH ROW EXECUTE FUNCTION messaging.hidden_dm_chats_deleted_user_cleanup();
