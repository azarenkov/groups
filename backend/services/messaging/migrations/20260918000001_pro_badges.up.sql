ALTER TABLE messaging.messaging_user_profiles
    ADD COLUMN tier character varying(16) DEFAULT 'free' NOT NULL,
    ADD COLUMN tier_valid_until timestamp with time zone,
    ADD COLUMN tier_updated_at timestamp with time zone,
    ADD COLUMN badge_style character varying(16) DEFAULT '' NOT NULL,
    ADD COLUMN badge_updated_at timestamp with time zone,
    ADD CONSTRAINT messaging_user_profiles_tier_check CHECK (tier IN ('free', 'pro', 'premium'));

CREATE INDEX messaging_user_profiles_paid_idx
    ON messaging.messaging_user_profiles (user_id)
    WHERE tier <> 'free';
