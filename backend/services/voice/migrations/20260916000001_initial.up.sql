CREATE SCHEMA IF NOT EXISTS voice;

CREATE TYPE voice.aggregate_type AS ENUM ('channel', 'server', 'user', 'message', 'dm_message', 'storage');

CREATE TABLE voice.call_evictions (
    call_id uuid NOT NULL,
    user_id uuid NOT NULL,
    attempts integer DEFAULT 0 NOT NULL,
    next_attempt_at timestamp with time zone DEFAULT now() NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT call_evictions_pkey PRIMARY KEY (call_id, user_id)
);

CREATE TABLE voice.call_invite_outcomes (
    call_id uuid NOT NULL,
    user_id uuid NOT NULL,
    inviter_id uuid NOT NULL,
    answered_at timestamp with time zone,
    declined boolean DEFAULT false NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    left_at timestamp with time zone
);

CREATE TABLE voice.call_participants (
    call_id uuid NOT NULL,
    user_id uuid NOT NULL,
    inviter_id uuid NOT NULL,
    status text NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    session_id text,
    answered_at timestamp with time zone,
    joined_at timestamp with time zone,
    CONSTRAINT call_participants_pkey PRIMARY KEY (call_id, user_id)
);

CREATE TABLE voice.calls (
    call_id uuid NOT NULL,
    video boolean DEFAULT false NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT calls_pkey PRIMARY KEY (call_id)
);

CREATE TABLE voice.voice_user_profiles (
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
    CONSTRAINT voice_user_profiles_pkey PRIMARY KEY (user_id)
);

CREATE TABLE voice.outbox (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    aggregate_id uuid NOT NULL,
    aggregate_type voice.aggregate_type NOT NULL,
    parent_id uuid,
    parent_type voice.aggregate_type,
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

CREATE INDEX call_evictions_due_idx ON voice.call_evictions (next_attempt_at);
CREATE INDEX idx_call_invite_outcomes_lookup ON voice.call_invite_outcomes (call_id, user_id, inviter_id, created_at DESC);
CREATE INDEX idx_call_participants_user ON voice.call_participants (user_id);
CREATE INDEX outbox_aggregate_idx ON voice.outbox (aggregate_type, aggregate_id, created_at);
CREATE INDEX outbox_pending_idx ON voice.outbox (last_attempt_at NULLS FIRST, created_at) WHERE (processed_at IS NULL);
CREATE INDEX outbox_processed_idx ON voice.outbox (processed_at) WHERE (processed_at IS NOT NULL);
