CREATE TABLE identity.voice_settings (
    user_id           UUID PRIMARY KEY,
    mic_volume        SMALLINT NOT NULL DEFAULT 100,
    output_volume     SMALLINT NOT NULL DEFAULT 100,
    noise_suppression BOOLEAN NOT NULL DEFAULT true,
    auto_sensitivity  BOOLEAN NOT NULL DEFAULT true,
    sensitivity       SMALLINT NOT NULL DEFAULT 50,
    push_to_talk      BOOLEAN NOT NULL DEFAULT false,
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT now(),

    CONSTRAINT voice_settings_mic_volume_check CHECK (mic_volume BETWEEN 0 AND 100),
    CONSTRAINT voice_settings_output_volume_check CHECK (output_volume BETWEEN 0 AND 100),
    CONSTRAINT voice_settings_sensitivity_check CHECK (sensitivity BETWEEN 0 AND 100)
);
