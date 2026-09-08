CREATE TABLE identity.hotkey_settings (
    user_id    UUID PRIMARY KEY,
    bindings   JSONB NOT NULL DEFAULT '{}'::jsonb,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
