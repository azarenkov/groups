CREATE TABLE voice.calls (
    call_id    UUID PRIMARY KEY,
    video      BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
