CREATE TABLE identity.user_entitlements (
    user_id uuid NOT NULL,
    tier character varying(16) DEFAULT 'free' NOT NULL,
    valid_until timestamp with time zone,
    source character varying(16) DEFAULT 'manual' NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT user_entitlements_pkey PRIMARY KEY (user_id),
    CONSTRAINT user_entitlements_tier_check CHECK (tier IN ('free', 'pro', 'premium')),
    CONSTRAINT user_entitlements_source_check CHECK (source IN ('manual', 'billing')),
    CONSTRAINT user_entitlements_user_id_fkey FOREIGN KEY (user_id) REFERENCES identity.users(id) ON DELETE CASCADE
);

ALTER TABLE identity.users
    ADD COLUMN badge_style character varying(16) DEFAULT '' NOT NULL,
    ADD CONSTRAINT users_badge_style_check CHECK (badge_style IN ('', 'tint', 'solid', 'neutral', 'outline', 'ink', 'etched', 'numbered'));

ALTER TABLE identity.voice_settings
    ADD COLUMN stream_quality character varying(10) DEFAULT '720p60' NOT NULL,
    ADD CONSTRAINT voice_settings_stream_quality_check CHECK (stream_quality IN ('720p60', '1080p60', '1440p60', '2160p60'));
