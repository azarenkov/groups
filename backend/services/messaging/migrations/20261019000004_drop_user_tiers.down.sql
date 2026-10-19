ALTER TABLE messaging.messaging_user_profiles
    ADD COLUMN badge_style character varying(16) DEFAULT '' NOT NULL,
    ADD COLUMN badge_updated_at timestamp with time zone;

CREATE TABLE messaging.messaging_user_tiers (
    user_id uuid NOT NULL,
    tier character varying(16) DEFAULT 'free' NOT NULL,
    valid_until timestamp with time zone,
    updated_at timestamp with time zone NOT NULL,
    member_number bigint,
    CONSTRAINT messaging_user_tiers_pkey PRIMARY KEY (user_id),
    CONSTRAINT messaging_user_tiers_tier_check CHECK (tier IN ('free', 'pro', 'premium'))
);

CREATE INDEX messaging_user_tiers_paid_idx
    ON messaging.messaging_user_tiers (user_id)
    WHERE tier <> 'free';
