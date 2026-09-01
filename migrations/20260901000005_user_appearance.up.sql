CREATE TABLE identity.user_settings (
    user_id        UUID PRIMARY KEY,
    theme          VARCHAR(10) NOT NULL DEFAULT 'system',
    accent         VARCHAR(10) NOT NULL DEFAULT '',
    chat_font_size SMALLINT NOT NULL DEFAULT 14,
    reduce_motion  BOOLEAN NOT NULL DEFAULT false,
    updated_at     TIMESTAMPTZ NOT NULL DEFAULT now(),

    CONSTRAINT user_settings_theme_check CHECK (theme IN ('light', 'dark', 'system')),
    CONSTRAINT user_settings_font_check CHECK (chat_font_size BETWEEN 12 AND 18)
);
