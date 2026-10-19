CREATE TABLE voice.voice_user_tiers (
    user_id uuid NOT NULL,
    tier character varying(16) DEFAULT 'free' NOT NULL,
    valid_until timestamp with time zone,
    updated_at timestamp with time zone NOT NULL,
    CONSTRAINT voice_user_tiers_pkey PRIMARY KEY (user_id),
    CONSTRAINT voice_user_tiers_tier_check CHECK (tier IN ('free', 'pro', 'premium'))
);
