CREATE TABLE messaging.notification_preferences (
    user_id    UUID NOT NULL,
    scope_type TEXT NOT NULL CHECK (scope_type IN ('channel', 'dm')),
    scope_id   UUID NOT NULL,
    level      TEXT NOT NULL CHECK (level IN ('all', 'mentions', 'none')),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, scope_type, scope_id)
);

CREATE INDEX idx_notification_preferences_user ON messaging.notification_preferences (user_id);
