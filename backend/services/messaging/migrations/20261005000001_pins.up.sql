CREATE TABLE messaging.channel_pinned_messages (
    message_id uuid NOT NULL,
    channel_id uuid NOT NULL,
    pinned_by uuid NOT NULL,
    pinned_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT channel_pinned_messages_pkey PRIMARY KEY (message_id),
    CONSTRAINT channel_pinned_messages_message_fkey FOREIGN KEY (message_id) REFERENCES messaging.messages(id) ON DELETE CASCADE,
    CONSTRAINT channel_pinned_messages_channel_fkey FOREIGN KEY (channel_id) REFERENCES messaging.channels(id) ON DELETE CASCADE
);

CREATE INDEX idx_channel_pinned_messages_channel ON messaging.channel_pinned_messages (channel_id, pinned_at DESC);

CREATE TABLE messaging.dm_pinned_messages (
    message_id uuid NOT NULL,
    user_low uuid NOT NULL,
    user_high uuid NOT NULL,
    pinned_by uuid NOT NULL,
    pinned_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT dm_pinned_messages_pkey PRIMARY KEY (message_id),
    CONSTRAINT dm_pinned_messages_message_fkey FOREIGN KEY (message_id) REFERENCES messaging.dm_messages(id) ON DELETE CASCADE,
    CONSTRAINT dm_pinned_messages_pair_check CHECK (user_low < user_high)
);

CREATE INDEX idx_dm_pinned_messages_pair ON messaging.dm_pinned_messages (user_low, user_high, pinned_at DESC);

CREATE TABLE messaging.conversation_pins (
    user_id uuid NOT NULL,
    kind text NOT NULL,
    target_id uuid NOT NULL,
    "position" integer NOT NULL,
    pinned_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT conversation_pins_pkey PRIMARY KEY (user_id, kind, target_id),
    CONSTRAINT conversation_pins_kind_check CHECK (kind = ANY (ARRAY['dm'::text, 'community'::text]))
);
