ALTER TABLE voice.voice_user_profiles
    ADD COLUMN tier character varying(16) DEFAULT 'free' NOT NULL,
    ADD COLUMN tier_valid_until timestamp with time zone,
    ADD COLUMN tier_updated_at timestamp with time zone,
    ADD CONSTRAINT voice_user_profiles_tier_check CHECK (tier IN ('free', 'pro', 'premium'));
