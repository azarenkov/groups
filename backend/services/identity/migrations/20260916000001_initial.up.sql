CREATE SCHEMA IF NOT EXISTS identity;

CREATE TYPE identity.verification_channel AS ENUM ('email', 'phone');

CREATE TYPE identity.verification_purpose AS ENUM ('registration', 'password_reset', 'email_change', 'phone_change');

CREATE TYPE identity.aggregate_type AS ENUM ('channel', 'server', 'user', 'message', 'dm_message', 'storage');

CREATE TABLE identity.connection_states (
    state uuid NOT NULL,
    user_id uuid NOT NULL,
    provider text NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT connection_states_pkey PRIMARY KEY (state)
);

CREATE TABLE identity.hotkey_settings (
    user_id uuid NOT NULL,
    bindings jsonb DEFAULT '{}'::jsonb NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT hotkey_settings_pkey PRIMARY KEY (user_id)
);

CREATE TABLE identity.notification_settings (
    user_id uuid NOT NULL,
    desktop_enabled boolean DEFAULT true NOT NULL,
    sounds_enabled boolean DEFAULT true NOT NULL,
    mentions character varying(10) DEFAULT 'all' NOT NULL,
    quiet_hours_enabled boolean DEFAULT false NOT NULL,
    quiet_start character varying(5) DEFAULT '22:00' NOT NULL,
    quiet_end character varying(5) DEFAULT '08:00' NOT NULL,
    calls_enabled boolean DEFAULT true NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT notification_settings_pkey PRIMARY KEY (user_id),
    CONSTRAINT notification_settings_mentions_check CHECK (mentions IN ('all', 'mentions', 'none'))
);

CREATE TABLE identity.users (
    id uuid NOT NULL,
    name character varying(100) NOT NULL,
    email character varying(255) NOT NULL,
    password_hash character varying(255) NOT NULL,
    email_verified_at timestamp with time zone,
    phone_verified_at timestamp with time zone,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    locale text DEFAULT 'en' NOT NULL,
    pending_email character varying(255),
    password_changed_at timestamp with time zone,
    disabled_at timestamp with time zone,
    deleted_at timestamp with time zone,
    bio character varying(190) DEFAULT '' NOT NULL,
    show_activity boolean DEFAULT true NOT NULL,
    timezone character varying(64) DEFAULT '' NOT NULL,
    show_game_activity boolean DEFAULT true NOT NULL,
    CONSTRAINT users_pkey PRIMARY KEY (id),
    CONSTRAINT users_email_key UNIQUE (email),
    CONSTRAINT users_name_key UNIQUE (name),
    CONSTRAINT users_locale_check CHECK (locale IN ('en', 'ru'))
);

CREATE TABLE identity.oauth_identities (
    provider text NOT NULL,
    subject text NOT NULL,
    user_id uuid NOT NULL,
    email text NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    display_name text,
    CONSTRAINT oauth_identities_pkey PRIMARY KEY (provider, subject),
    CONSTRAINT oauth_identities_user_id_fkey FOREIGN KEY (user_id) REFERENCES identity.users(id) ON DELETE CASCADE
);

CREATE TABLE identity.refresh_tokens (
    id uuid NOT NULL,
    user_id uuid NOT NULL,
    expires_at timestamp with time zone NOT NULL,
    revoked_at timestamp with time zone,
    last_ip text,
    last_user_agent text,
    last_used_at timestamp with time zone DEFAULT now() NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT refresh_tokens_pkey PRIMARY KEY (id),
    CONSTRAINT refresh_tokens_user_id_fkey FOREIGN KEY (user_id) REFERENCES identity.users(id) ON DELETE CASCADE,
    CONSTRAINT refresh_tokens_expires_after_created_check CHECK ((expires_at > created_at))
);

CREATE TABLE identity.user_settings (
    user_id uuid NOT NULL,
    theme character varying(10) DEFAULT 'system' NOT NULL,
    accent character varying(10) DEFAULT '' NOT NULL,
    chat_font_size smallint DEFAULT 14 NOT NULL,
    reduce_motion boolean DEFAULT false NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT user_settings_pkey PRIMARY KEY (user_id),
    CONSTRAINT user_settings_font_check CHECK (((chat_font_size >= 12) AND (chat_font_size <= 18))),
    CONSTRAINT user_settings_theme_check CHECK (theme IN ('light', 'dark', 'system'))
);

CREATE TABLE identity.verification_codes (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    user_id uuid NOT NULL,
    channel identity.verification_channel NOT NULL,
    code_hash character varying(255) NOT NULL,
    purpose identity.verification_purpose NOT NULL,
    expires_at timestamp with time zone NOT NULL,
    consumed_at timestamp with time zone,
    attempts smallint DEFAULT 0 NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT verification_codes_pkey PRIMARY KEY (id),
    CONSTRAINT verification_codes_user_id_fkey FOREIGN KEY (user_id) REFERENCES identity.users(id) ON DELETE CASCADE,
    CONSTRAINT verification_codes_attempts_check CHECK (((attempts >= 0) AND (attempts <= 10))),
    CONSTRAINT verification_codes_expires_after_created_check CHECK ((expires_at > created_at))
);

CREATE TABLE identity.voice_settings (
    user_id uuid NOT NULL,
    mic_volume smallint DEFAULT 100 NOT NULL,
    output_volume smallint DEFAULT 100 NOT NULL,
    auto_sensitivity boolean DEFAULT true NOT NULL,
    sensitivity smallint DEFAULT 50 NOT NULL,
    push_to_talk boolean DEFAULT false NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    noise_mode character varying(10) DEFAULT 'standard' NOT NULL,
    ptt_key character varying(32) DEFAULT 'KeyM' NOT NULL,
    CONSTRAINT voice_settings_pkey PRIMARY KEY (user_id),
    CONSTRAINT voice_settings_mic_volume_check CHECK (((mic_volume >= 0) AND (mic_volume <= 100))),
    CONSTRAINT voice_settings_noise_mode_check CHECK (noise_mode IN ('off', 'standard', 'high')),
    CONSTRAINT voice_settings_output_volume_check CHECK (((output_volume >= 0) AND (output_volume <= 100))),
    CONSTRAINT voice_settings_sensitivity_check CHECK (((sensitivity >= 0) AND (sensitivity <= 100)))
);

CREATE TABLE identity.outbox (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    aggregate_id uuid NOT NULL,
    aggregate_type identity.aggregate_type NOT NULL,
    parent_id uuid,
    parent_type identity.aggregate_type,
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

CREATE UNIQUE INDEX uq_oauth_identities_provider_user ON identity.oauth_identities (provider, user_id);
CREATE INDEX outbox_aggregate_idx ON identity.outbox (aggregate_type, aggregate_id, created_at);
CREATE INDEX outbox_pending_idx ON identity.outbox (last_attempt_at NULLS FIRST, created_at) WHERE (processed_at IS NULL);
CREATE INDEX outbox_processed_idx ON identity.outbox (processed_at) WHERE (processed_at IS NOT NULL);
CREATE INDEX refresh_tokens_expires_idx ON identity.refresh_tokens (expires_at) WHERE (revoked_at IS NULL);
CREATE INDEX refresh_tokens_user_id_active_idx ON identity.refresh_tokens (user_id) WHERE (revoked_at IS NULL);
CREATE INDEX refresh_tokens_user_id_last_used_idx ON identity.refresh_tokens (user_id, last_used_at DESC) WHERE (revoked_at IS NULL);
CREATE INDEX idx_users_email ON identity.users (email);
CREATE UNIQUE INDEX users_name_lower_key ON identity.users (lower(((name)::text COLLATE "C")));
CREATE INDEX verification_codes_expires_idx ON identity.verification_codes (expires_at) WHERE (consumed_at IS NULL);
CREATE INDEX verification_codes_lookup_idx ON identity.verification_codes (user_id, channel, purpose, created_at DESC);
