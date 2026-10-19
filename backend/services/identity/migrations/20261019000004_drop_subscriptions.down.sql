ALTER TABLE identity.users
    ADD COLUMN badge_style character varying(16) DEFAULT '' NOT NULL,
    ADD CONSTRAINT users_badge_style_check CHECK (badge_style IN ('', 'tint', 'solid', 'neutral', 'outline', 'ink', 'etched', 'numbered', 'number_only', 'gold'));

CREATE TABLE identity.connection_states (
    state uuid NOT NULL,
    user_id uuid NOT NULL,
    provider text NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT connection_states_pkey PRIMARY KEY (state)
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

CREATE UNIQUE INDEX uq_oauth_identities_provider_user ON identity.oauth_identities (provider, user_id);

CREATE SEQUENCE identity.pro_member_number_seq;

CREATE TABLE identity.user_entitlements (
    user_id uuid NOT NULL,
    tier character varying(16) DEFAULT 'free' NOT NULL,
    valid_until timestamp with time zone,
    source character varying(16) DEFAULT 'manual' NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    member_number bigint,
    trial_used_at timestamp with time zone,
    trial_reminded_at timestamp with time zone,
    CONSTRAINT user_entitlements_pkey PRIMARY KEY (user_id),
    CONSTRAINT user_entitlements_tier_check CHECK (tier IN ('free', 'pro', 'premium')),
    CONSTRAINT user_entitlements_source_check CHECK (source IN ('manual', 'billing', 'trial', 'expiry')),
    CONSTRAINT user_entitlements_user_id_fkey FOREIGN KEY (user_id) REFERENCES identity.users(id) ON DELETE CASCADE
);

CREATE UNIQUE INDEX user_entitlements_member_number_idx
    ON identity.user_entitlements (member_number)
    WHERE member_number IS NOT NULL;

CREATE INDEX user_entitlements_expiring_idx
    ON identity.user_entitlements (valid_until)
    WHERE tier <> 'free' AND valid_until IS NOT NULL;

CREATE TABLE identity.trial_grants (
    id bigserial NOT NULL,
    user_id uuid,
    email_hash character(64) NOT NULL,
    ip_hash character(64),
    granted_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT trial_grants_pkey PRIMARY KEY (id),
    CONSTRAINT trial_grants_user_id_fkey FOREIGN KEY (user_id) REFERENCES identity.users(id) ON DELETE SET NULL
);

CREATE UNIQUE INDEX trial_grants_email_hash_idx ON identity.trial_grants (email_hash);
CREATE INDEX trial_grants_ip_hash_idx ON identity.trial_grants (ip_hash, granted_at) WHERE ip_hash IS NOT NULL;
