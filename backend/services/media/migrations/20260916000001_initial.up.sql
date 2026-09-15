CREATE SCHEMA IF NOT EXISTS media;

CREATE TABLE media.attachments (
    id uuid NOT NULL,
    owner_id uuid NOT NULL,
    file_name text NOT NULL,
    content_type text NOT NULL,
    size_bytes bigint NOT NULL,
    storage_key text NOT NULL,
    deleting_at timestamp with time zone,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    claimed_at timestamp with time zone,
    claim_token uuid,
    CONSTRAINT attachments_pkey PRIMARY KEY (id)
);

CREATE TABLE media.media_deleted_users (
    user_id uuid NOT NULL,
    deleted_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT media_deleted_users_pkey PRIMARY KEY (user_id)
);

CREATE TABLE media.media_server_logos (
    server_id uuid NOT NULL,
    storage_key text NOT NULL,
    CONSTRAINT media_server_logos_pkey PRIMARY KEY (server_id)
);

CREATE TABLE media.media_server_owners (
    server_id uuid NOT NULL,
    owner_id uuid NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    deleted_at timestamp with time zone,
    CONSTRAINT media_server_owners_pkey PRIMARY KEY (server_id)
);

CREATE TABLE media.media_user_avatars (
    user_id uuid NOT NULL,
    storage_key text NOT NULL,
    CONSTRAINT media_user_avatars_pkey PRIMARY KEY (user_id)
);

CREATE TABLE media.storage_cleanup_queue (
    storage_key text NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT storage_cleanup_queue_pkey PRIMARY KEY (storage_key)
);

CREATE INDEX idx_attachments_unclaimed ON media.attachments (created_at) WHERE ((claimed_at IS NULL) AND (deleting_at IS NULL));
