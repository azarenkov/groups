CREATE TABLE media.media_user_banners (
    user_id uuid NOT NULL,
    storage_key text NOT NULL,
    CONSTRAINT media_user_banners_pkey PRIMARY KEY (user_id)
);

CREATE TABLE media.media_user_tiers (
    user_id uuid NOT NULL,
    tier character varying(16) DEFAULT 'free' NOT NULL,
    valid_until timestamp with time zone,
    updated_at timestamp with time zone NOT NULL,
    CONSTRAINT media_user_tiers_pkey PRIMARY KEY (user_id),
    CONSTRAINT media_user_tiers_tier_check CHECK (tier IN ('free', 'pro', 'premium'))
);
