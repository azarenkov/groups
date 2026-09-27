ALTER TABLE messaging.message_attachments
    ADD COLUMN voice_duration_ms integer,
    ADD COLUMN voice_waveform bytea;

ALTER TABLE messaging.dm_message_attachments
    ADD COLUMN voice_duration_ms integer,
    ADD COLUMN voice_waveform bytea;

CREATE TABLE messaging.voice_listens (
    message_id uuid NOT NULL,
    user_id uuid NOT NULL,
    listened_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT voice_listens_pkey PRIMARY KEY (message_id, user_id)
);
