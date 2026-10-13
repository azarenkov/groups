CREATE TABLE messaging.messaging_user_tiers (
    user_id uuid NOT NULL,
    tier character varying(16) DEFAULT 'free' NOT NULL,
    valid_until timestamp with time zone,
    updated_at timestamp with time zone NOT NULL,
    member_number bigint,
    CONSTRAINT messaging_user_tiers_pkey PRIMARY KEY (user_id),
    CONSTRAINT messaging_user_tiers_tier_check CHECK (tier IN ('free', 'pro', 'premium'))
);

INSERT INTO messaging.messaging_user_tiers (user_id, tier, valid_until, updated_at, member_number)
SELECT user_id, tier, tier_valid_until, COALESCE(tier_updated_at, to_timestamp(0)), member_number
FROM messaging.messaging_user_profiles
WHERE tier_updated_at IS NOT NULL OR tier <> 'free' OR member_number IS NOT NULL;

CREATE INDEX messaging_user_tiers_paid_idx
    ON messaging.messaging_user_tiers (user_id)
    WHERE tier <> 'free';
