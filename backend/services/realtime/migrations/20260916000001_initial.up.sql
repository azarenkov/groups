CREATE SCHEMA IF NOT EXISTS realtime;

CREATE TABLE realtime.realtime_deleted_servers (
    server_id uuid NOT NULL,
    deleted_at timestamp with time zone NOT NULL,
    CONSTRAINT realtime_deleted_servers_pkey PRIMARY KEY (server_id)
);

CREATE TABLE realtime.realtime_dm_pairs (
    user_a uuid NOT NULL,
    user_b uuid NOT NULL,
    CONSTRAINT realtime_dm_pairs_pkey PRIMARY KEY (user_a, user_b)
);

CREATE TABLE realtime.realtime_friend_edges (
    user_id uuid NOT NULL,
    friend_id uuid NOT NULL,
    removed boolean DEFAULT false NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    pending boolean DEFAULT false NOT NULL,
    CONSTRAINT realtime_friend_edges_pkey PRIMARY KEY (user_id, friend_id)
);

CREATE TABLE realtime.realtime_server_members (
    server_id uuid NOT NULL,
    user_id uuid NOT NULL,
    removed boolean DEFAULT false NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT realtime_server_members_pkey PRIMARY KEY (server_id, user_id)
);

CREATE TABLE realtime.realtime_user_blocks (
    blocker_id uuid NOT NULL,
    blocked_id uuid NOT NULL,
    active boolean DEFAULT true NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT realtime_user_blocks_pkey PRIMARY KEY (blocker_id, blocked_id)
);

CREATE TABLE realtime.realtime_user_names (
    user_id uuid NOT NULL,
    name text NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT realtime_user_names_pkey PRIMARY KEY (user_id)
);

CREATE INDEX realtime_server_members_user_idx ON realtime.realtime_server_members (user_id);
CREATE INDEX realtime_user_blocks_blocked_idx ON realtime.realtime_user_blocks (blocked_id);
