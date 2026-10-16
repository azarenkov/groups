CREATE TABLE voice.voice_media_states (
    user_id uuid NOT NULL,
    mic_muted boolean NOT NULL,
    deafened boolean NOT NULL,
    source uuid,
    seq bigint,
    updated_at timestamp with time zone NOT NULL,
    CONSTRAINT voice_media_states_pkey PRIMARY KEY (user_id)
);
