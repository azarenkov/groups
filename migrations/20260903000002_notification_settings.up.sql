CREATE TABLE identity.notification_settings (
    user_id             UUID PRIMARY KEY,
    desktop_enabled     BOOLEAN NOT NULL DEFAULT true,
    sounds_enabled      BOOLEAN NOT NULL DEFAULT true,
    mentions            VARCHAR(10) NOT NULL DEFAULT 'all',
    quiet_hours_enabled BOOLEAN NOT NULL DEFAULT false,
    quiet_start         VARCHAR(5) NOT NULL DEFAULT '22:00',
    quiet_end           VARCHAR(5) NOT NULL DEFAULT '08:00',
    calls_enabled       BOOLEAN NOT NULL DEFAULT true,
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT now(),

    CONSTRAINT notification_settings_mentions_check CHECK (mentions IN ('all', 'mentions', 'none'))
);
