CREATE TABLE voice.voice_user_tiers (
    user_id uuid NOT NULL,
    tier character varying(16) DEFAULT 'free' NOT NULL,
    valid_until timestamp with time zone,
    updated_at timestamp with time zone NOT NULL,
    CONSTRAINT voice_user_tiers_pkey PRIMARY KEY (user_id),
    CONSTRAINT voice_user_tiers_tier_check CHECK (tier IN ('free', 'pro', 'premium'))
);

INSERT INTO voice.voice_user_tiers (user_id, tier, valid_until, updated_at)
SELECT user_id, tier, tier_valid_until, COALESCE(tier_updated_at, to_timestamp(0))
FROM voice.voice_user_profiles
WHERE tier_updated_at IS NOT NULL OR tier <> 'free';

ALTER TABLE voice.voice_user_profiles
    DROP CONSTRAINT IF EXISTS voice_user_profiles_tier_check,
    DROP COLUMN IF EXISTS tier_updated_at,
    DROP COLUMN IF EXISTS tier_valid_until,
    DROP COLUMN IF EXISTS tier;
