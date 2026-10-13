ALTER TABLE messaging.messaging_user_profiles
    ADD COLUMN tier character varying(16) DEFAULT 'free' NOT NULL,
    ADD COLUMN tier_valid_until timestamp with time zone,
    ADD COLUMN tier_updated_at timestamp with time zone,
    ADD COLUMN member_number bigint,
    ADD CONSTRAINT messaging_user_profiles_tier_check CHECK (tier IN ('free', 'pro', 'premium'));

UPDATE messaging.messaging_user_profiles p
SET tier = t.tier,
    tier_valid_until = t.valid_until,
    tier_updated_at = t.updated_at,
    member_number = t.member_number
FROM messaging.messaging_user_tiers t
WHERE t.user_id = p.user_id;

CREATE INDEX messaging_user_profiles_paid_idx
    ON messaging.messaging_user_profiles (user_id)
    WHERE tier <> 'free';

DROP TABLE IF EXISTS messaging.messaging_user_tiers;
