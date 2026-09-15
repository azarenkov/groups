CREATE SCHEMA IF NOT EXISTS messaging;

CREATE TYPE messaging.friendship_status AS ENUM ('pending_outgoing', 'pending_incoming', 'accepted', 'declined');

CREATE TYPE messaging.kind AS ENUM ('text', 'voice');

CREATE TYPE messaging.visibility AS ENUM ('public', 'private');

CREATE TYPE messaging.webhook_source AS ENUM ('livekit', 'livekit_pooler');

CREATE TYPE messaging.aggregate_type AS ENUM ('channel', 'server', 'user', 'message', 'dm_message', 'storage');

CREATE TABLE messaging.call_log_consumed (
    call_id uuid NOT NULL,
    recipient_id uuid NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    sender_id uuid,
    CONSTRAINT call_log_consumed_pair_unique UNIQUE (call_id, sender_id, recipient_id)
);

CREATE TABLE messaging.servers (
    id uuid NOT NULL,
    name character varying(100) NOT NULL,
    owner_id uuid NOT NULL,
    description text,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    accent_color character varying(7),
    CONSTRAINT servers_pkey PRIMARY KEY (id),
    CONSTRAINT servers_name_key UNIQUE (name)
);

CREATE TABLE messaging.channels (
    id uuid NOT NULL,
    server_id uuid NOT NULL,
    owner_id uuid NOT NULL,
    name character varying(100) NOT NULL,
    kind messaging.kind DEFAULT 'text'::messaging.kind NOT NULL,
    visibility messaging.visibility DEFAULT 'public'::messaging.visibility NOT NULL,
    password_hash character varying(255),
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    last_join timestamp with time zone DEFAULT now() NOT NULL,
    is_default boolean DEFAULT false NOT NULL,
    CONSTRAINT channels_pkey PRIMARY KEY (id),
    CONSTRAINT channels_server_id_name_key UNIQUE (server_id, name),
    CONSTRAINT channels_server_id_fkey FOREIGN KEY (server_id) REFERENCES messaging.servers(id) ON DELETE CASCADE,
    CONSTRAINT check_private_channel_password CHECK ((((visibility = 'private'::messaging.visibility) AND (password_hash IS NOT NULL)) OR (visibility = 'public'::messaging.visibility)))
);

CREATE TABLE messaging.channel_deafened_participants (
    channel_id uuid NOT NULL,
    user_id uuid NOT NULL,
    deafened_by uuid NOT NULL,
    deafened_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT channel_deafened_participants_pkey PRIMARY KEY (channel_id, user_id),
    CONSTRAINT channel_deafened_participants_channel_id_fkey FOREIGN KEY (channel_id) REFERENCES messaging.channels(id) ON DELETE CASCADE
);

CREATE TABLE messaging.channel_muted_participants (
    channel_id uuid NOT NULL,
    user_id uuid NOT NULL,
    muted_by uuid NOT NULL,
    muted_at timestamp with time zone DEFAULT now() NOT NULL,
    self_muted boolean DEFAULT false NOT NULL,
    CONSTRAINT channel_muted_participants_pkey PRIMARY KEY (channel_id, user_id),
    CONSTRAINT channel_muted_participants_channel_id_fkey FOREIGN KEY (channel_id) REFERENCES messaging.channels(id) ON DELETE CASCADE
);

CREATE TABLE messaging.channel_read_state (
    user_id uuid NOT NULL,
    channel_id uuid NOT NULL,
    last_read_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT channel_read_state_pkey PRIMARY KEY (user_id, channel_id),
    CONSTRAINT channel_read_state_channel_id_fkey FOREIGN KEY (channel_id) REFERENCES messaging.channels(id) ON DELETE CASCADE
);

CREATE TABLE messaging.channel_stream_viewers (
    channel_id uuid NOT NULL,
    streamer_id uuid NOT NULL,
    viewer_id uuid NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    viewer_session_id text NOT NULL,
    CONSTRAINT channel_stream_viewers_pkey PRIMARY KEY (channel_id, streamer_id, viewer_id, viewer_session_id),
    CONSTRAINT channel_stream_viewers_channel_id_fkey FOREIGN KEY (channel_id) REFERENCES messaging.channels(id) ON DELETE CASCADE
);

CREATE TABLE messaging.channel_text_participants (
    channel_id uuid NOT NULL,
    user_id uuid NOT NULL,
    joined_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT channel_text_participants_pkey PRIMARY KEY (channel_id, user_id),
    CONSTRAINT channel_text_participants_channel_id_fkey FOREIGN KEY (channel_id) REFERENCES messaging.channels(id) ON DELETE CASCADE
);

CREATE TABLE messaging.channel_voice_participants (
    channel_id uuid NOT NULL,
    user_id uuid NOT NULL,
    session_id text NOT NULL,
    joined_at timestamp with time zone DEFAULT now() NOT NULL,
    camera_on boolean DEFAULT false NOT NULL,
    screen_on boolean DEFAULT false NOT NULL,
    CONSTRAINT channel_voice_participants_pkey PRIMARY KEY (channel_id, user_id, session_id),
    CONSTRAINT channel_voice_participants_channel_id_fkey FOREIGN KEY (channel_id) REFERENCES messaging.channels(id) ON DELETE CASCADE
);

CREATE TABLE messaging.channel_voice_visits (
    channel_id uuid NOT NULL,
    user_id uuid NOT NULL,
    last_joined_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT channel_voice_visits_pkey PRIMARY KEY (channel_id, user_id),
    CONSTRAINT channel_voice_visits_channel_id_fkey FOREIGN KEY (channel_id) REFERENCES messaging.channels(id) ON DELETE CASCADE
);

CREATE TABLE messaging.dm_conversations (
    user_id uuid NOT NULL,
    peer_id uuid NOT NULL,
    last_message_id uuid NOT NULL,
    last_created_at timestamp with time zone NOT NULL,
    CONSTRAINT dm_conversations_pkey PRIMARY KEY (user_id, peer_id)
);

CREATE TABLE messaging.dm_message_attachments (
    dm_message_id uuid NOT NULL,
    attachment_id uuid NOT NULL,
    file_name text NOT NULL,
    content_type text NOT NULL,
    size_bytes bigint NOT NULL,
    storage_key text NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT dm_message_attachments_pkey PRIMARY KEY (dm_message_id, attachment_id)
);

CREATE TABLE messaging.dm_messages (
    id uuid NOT NULL,
    sender_id uuid NOT NULL,
    recipient_id uuid NOT NULL,
    content text NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    edited_at timestamp with time zone,
    search_tsv tsvector GENERATED ALWAYS AS (to_tsvector('simple'::regconfig, content)) STORED,
    invite_server_id uuid,
    call_status text,
    call_duration_seconds integer,
    call_id uuid,
    CONSTRAINT dm_messages_pkey PRIMARY KEY (id),
    CONSTRAINT dm_messages_invite_server_id_fkey FOREIGN KEY (invite_server_id) REFERENCES messaging.servers(id) ON DELETE CASCADE,
    CONSTRAINT dm_messages_check CHECK ((sender_id <> recipient_id))
);

CREATE TABLE messaging.dm_read_state (
    user_id uuid NOT NULL,
    friend_id uuid NOT NULL,
    last_read_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT dm_read_state_pkey PRIMARY KEY (user_id, friend_id)
);

CREATE TABLE messaging.friendships (
    user_id uuid NOT NULL,
    friend_id uuid NOT NULL,
    status messaging.friendship_status NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT friendships_pkey PRIMARY KEY (user_id, friend_id),
    CONSTRAINT friendships_check CHECK ((user_id <> friend_id))
);

CREATE TABLE messaging.message_attachments (
    message_id uuid NOT NULL,
    attachment_id uuid NOT NULL,
    file_name text NOT NULL,
    content_type text NOT NULL,
    size_bytes bigint NOT NULL,
    storage_key text NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT message_attachments_pkey PRIMARY KEY (message_id, attachment_id)
);

CREATE TABLE messaging.messages (
    id uuid NOT NULL,
    channel_id uuid NOT NULL,
    author_id uuid NOT NULL,
    content text NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    edited_at timestamp with time zone,
    search_tsv tsvector GENERATED ALWAYS AS (to_tsvector('simple'::regconfig, content)) STORED,
    mentions uuid[] DEFAULT '{}'::uuid[] NOT NULL,
    kind text DEFAULT 'user' NOT NULL,
    system_payload jsonb,
    CONSTRAINT messages_pkey PRIMARY KEY (id),
    CONSTRAINT messages_channel_id_fkey FOREIGN KEY (channel_id) REFERENCES messaging.channels(id) ON DELETE CASCADE,
    CONSTRAINT messages_kind_check CHECK (kind IN ('user', 'system'))
);

CREATE TABLE messaging.messaging_deleted_users (
    user_id uuid NOT NULL,
    deleted_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT messaging_deleted_users_pkey PRIMARY KEY (user_id)
);

CREATE TABLE messaging.messaging_user_profiles (
    user_id uuid NOT NULL,
    name text NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    active boolean DEFAULT true NOT NULL,
    bio character varying(190) DEFAULT '' NOT NULL,
    show_activity boolean DEFAULT true NOT NULL,
    profile_updated_at timestamp with time zone,
    active_updated_at timestamp with time zone DEFAULT to_timestamp((0)::double precision) NOT NULL,
    timezone character varying(64) DEFAULT '' NOT NULL,
    registered_at timestamp with time zone,
    CONSTRAINT messaging_user_profiles_pkey PRIMARY KEY (user_id)
);

CREATE TABLE messaging.notification_preferences (
    user_id uuid NOT NULL,
    scope_type text NOT NULL,
    scope_id uuid NOT NULL,
    level text NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    version bigint DEFAULT 1 NOT NULL,
    CONSTRAINT notification_preferences_pkey PRIMARY KEY (user_id, scope_type, scope_id),
    CONSTRAINT notification_preferences_level_check CHECK (level IN ('all', 'mentions', 'none')),
    CONSTRAINT notification_preferences_scope_type_check CHECK (scope_type IN ('channel', 'dm'))
);

CREATE TABLE messaging.server_invite_links (
    token text NOT NULL,
    server_id uuid NOT NULL,
    created_by uuid NOT NULL,
    name character varying(60),
    is_primary boolean DEFAULT false NOT NULL,
    max_uses integer,
    uses integer DEFAULT 0 NOT NULL,
    expires_at timestamp with time zone,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    revoked_at timestamp with time zone,
    CONSTRAINT server_invite_links_pkey PRIMARY KEY (token),
    CONSTRAINT server_invite_links_server_id_fkey FOREIGN KEY (server_id) REFERENCES messaging.servers(id) ON DELETE CASCADE
);

CREATE TABLE messaging.server_members (
    server_id uuid NOT NULL,
    user_id uuid NOT NULL,
    joined_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT server_members_pkey PRIMARY KEY (server_id, user_id),
    CONSTRAINT server_members_server_id_fkey FOREIGN KEY (server_id) REFERENCES messaging.servers(id) ON DELETE CASCADE
);

CREATE TABLE messaging.server_roles (
    id uuid NOT NULL,
    server_id uuid NOT NULL,
    name text NOT NULL,
    color text DEFAULT '#8e8e93' NOT NULL,
    permissions bigint DEFAULT 0 NOT NULL,
    "position" integer DEFAULT 0 NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT server_roles_pkey PRIMARY KEY (id),
    CONSTRAINT server_roles_server_id_fkey FOREIGN KEY (server_id) REFERENCES messaging.servers(id) ON DELETE CASCADE
);

CREATE TABLE messaging.server_member_roles (
    server_id uuid NOT NULL,
    user_id uuid NOT NULL,
    role_id uuid NOT NULL,
    CONSTRAINT server_member_roles_pkey PRIMARY KEY (server_id, user_id, role_id),
    CONSTRAINT server_member_roles_role_id_fkey FOREIGN KEY (role_id) REFERENCES messaging.server_roles(id) ON DELETE CASCADE,
    CONSTRAINT server_member_roles_server_id_user_id_fkey FOREIGN KEY (server_id, user_id) REFERENCES messaging.server_members(server_id, user_id) ON DELETE CASCADE
);

CREATE TABLE messaging.user_blocks (
    blocker_id uuid NOT NULL,
    blocked_id uuid NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT user_blocks_pkey PRIMARY KEY (blocker_id, blocked_id),
    CONSTRAINT user_blocks_check CHECK ((blocker_id <> blocked_id))
);

CREATE TABLE messaging.user_privacy_settings (
    user_id uuid NOT NULL,
    dm_policy character varying(10) DEFAULT 'friends' NOT NULL,
    accept_friend_requests boolean DEFAULT true NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    call_policy character varying(10) DEFAULT 'friends' NOT NULL,
    CONSTRAINT user_privacy_settings_pkey PRIMARY KEY (user_id),
    CONSTRAINT user_privacy_call_policy_check CHECK (call_policy IN ('all', 'friends', 'no_one')),
    CONSTRAINT user_privacy_dm_policy_check CHECK (dm_policy IN ('all', 'friends', 'no_one'))
);

CREATE TABLE messaging.webhook_inbox (
    source messaging.webhook_source NOT NULL,
    external_id text NOT NULL,
    processed_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT webhook_inbox_pkey PRIMARY KEY (source, external_id)
);

CREATE TABLE messaging.outbox (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    aggregate_id uuid NOT NULL,
    aggregate_type messaging.aggregate_type NOT NULL,
    parent_id uuid,
    parent_type messaging.aggregate_type,
    event_type character varying(128) NOT NULL,
    payload jsonb NOT NULL,
    occurred_at timestamp with time zone NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    processed_at timestamp with time zone,
    last_attempt_at timestamp with time zone,
    attempts smallint DEFAULT 0 NOT NULL,
    last_error text,
    request_id text,
    CONSTRAINT outbox_pkey PRIMARY KEY (id),
    CONSTRAINT outbox_attempts_check CHECK (((attempts >= 0) AND (attempts <= 100))),
    CONSTRAINT outbox_parent_consistency CHECK ((((parent_id IS NULL) AND (parent_type IS NULL)) OR ((parent_id IS NOT NULL) AND (parent_type IS NOT NULL))))
);

CREATE INDEX idx_channel_deafened_participants_user_id ON messaging.channel_deafened_participants (user_id);
CREATE INDEX idx_channel_muted_participants_user_id ON messaging.channel_muted_participants (user_id);
CREATE INDEX idx_channel_read_state_user ON messaging.channel_read_state (user_id);
CREATE INDEX idx_channel_stream_viewers_lookup ON messaging.channel_stream_viewers (channel_id, streamer_id);
CREATE INDEX idx_channel_text_participants_user ON messaging.channel_text_participants (user_id);
CREATE INDEX idx_channel_voice_participants_user_id ON messaging.channel_voice_participants (user_id);
CREATE INDEX idx_channel_voice_visits_user_recent ON messaging.channel_voice_visits (user_id, last_joined_at DESC);
CREATE INDEX idx_channels_kind ON messaging.channels (kind);
CREATE INDEX idx_channels_last_join ON messaging.channels (last_join DESC);
CREATE INDEX idx_channels_name ON messaging.channels (name);
CREATE INDEX idx_channels_owner_id ON messaging.channels (owner_id);
CREATE INDEX idx_channels_server_id ON messaging.channels (server_id);
CREATE INDEX idx_channels_visibility ON messaging.channels (visibility);
CREATE UNIQUE INDEX uq_channels_default_per_server ON messaging.channels (server_id) WHERE is_default;
CREATE INDEX idx_dm_conversations_user_cursor ON messaging.dm_conversations (user_id, last_created_at DESC, last_message_id DESC);
CREATE INDEX idx_dm_messages_invite_server ON messaging.dm_messages (invite_server_id) WHERE (invite_server_id IS NOT NULL);
CREATE INDEX idx_dm_messages_pair_created ON messaging.dm_messages (LEAST(sender_id, recipient_id), GREATEST(sender_id, recipient_id), created_at DESC);
CREATE INDEX idx_dm_messages_search ON messaging.dm_messages USING gin (search_tsv);
CREATE UNIQUE INDEX dm_messages_invite_unique ON messaging.dm_messages (sender_id, recipient_id, invite_server_id) WHERE (invite_server_id IS NOT NULL);
CREATE INDEX idx_dm_read_state_user ON messaging.dm_read_state (user_id);
CREATE INDEX idx_friendships_user_cursor ON messaging.friendships (user_id, updated_at DESC, friend_id DESC);
CREATE INDEX idx_friendships_user_status ON messaging.friendships (user_id, status);
CREATE INDEX idx_friendships_user_status_cursor ON messaging.friendships (user_id, status, updated_at DESC, friend_id DESC);
CREATE INDEX idx_messages_author ON messaging.messages (author_id);
CREATE INDEX idx_messages_channel_created ON messaging.messages (channel_id, created_at DESC);
CREATE INDEX idx_messages_search_tsv ON messaging.messages USING gin (search_tsv);
CREATE INDEX idx_notification_preferences_user ON messaging.notification_preferences (user_id);
CREATE INDEX outbox_aggregate_idx ON messaging.outbox (aggregate_type, aggregate_id, created_at);
CREATE INDEX outbox_pending_idx ON messaging.outbox (last_attempt_at NULLS FIRST, created_at) WHERE (processed_at IS NULL);
CREATE INDEX outbox_processed_idx ON messaging.outbox (processed_at) WHERE (processed_at IS NOT NULL);
CREATE INDEX idx_server_invite_links_server ON messaging.server_invite_links (server_id);
CREATE INDEX idx_server_invite_links_server_cursor ON messaging.server_invite_links (server_id, created_at DESC, token DESC) WHERE ((NOT is_primary) AND (revoked_at IS NULL));
CREATE UNIQUE INDEX uq_server_invite_links_primary ON messaging.server_invite_links (server_id) WHERE (is_primary AND (revoked_at IS NULL));
CREATE INDEX idx_server_member_roles_user ON messaging.server_member_roles (server_id, user_id);
CREATE INDEX idx_server_members_server_id ON messaging.server_members (server_id);
CREATE INDEX idx_server_members_user_id ON messaging.server_members (user_id);
CREATE INDEX idx_server_roles_server ON messaging.server_roles (server_id);
CREATE INDEX idx_servers_name ON messaging.servers (name);
CREATE INDEX idx_servers_owner_id ON messaging.servers (owner_id);
CREATE INDEX idx_user_blocks_blocked ON messaging.user_blocks (blocked_id);
CREATE INDEX idx_user_blocks_blocker_cursor ON messaging.user_blocks (blocker_id, created_at DESC, blocked_id DESC);
CREATE INDEX webhook_inbox_processed_at_idx ON messaging.webhook_inbox (processed_at);

CREATE FUNCTION messaging.dm_conversations_sync() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
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

CREATE TRIGGER dm_conversations_sync AFTER INSERT OR DELETE ON messaging.dm_messages FOR EACH ROW EXECUTE FUNCTION messaging.dm_conversations_sync();
