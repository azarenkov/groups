ALTER TABLE voice.voice_user_profiles
    ADD COLUMN bio character varying(190) DEFAULT '' NOT NULL,
    ADD COLUMN show_activity boolean DEFAULT true NOT NULL,
    ADD COLUMN timezone character varying(64) DEFAULT '' NOT NULL,
    ADD COLUMN registered_at timestamp with time zone,
    ADD COLUMN profile_updated_at timestamp with time zone;
