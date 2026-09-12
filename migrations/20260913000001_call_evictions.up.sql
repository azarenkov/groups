CREATE TABLE voice.call_evictions (
    call_id         UUID NOT NULL,
    user_id         UUID NOT NULL,
    attempts        INT NOT NULL DEFAULT 0,
    next_attempt_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (call_id, user_id)
);
CREATE INDEX call_evictions_due_idx ON voice.call_evictions (next_attempt_at);
