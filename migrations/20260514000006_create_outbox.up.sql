CREATE TYPE aggregate_type AS ENUM (
    'channel',
    'server',
    'user'
);

CREATE TABLE outbox (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    aggregate_id    UUID NOT NULL,
    aggregate_type  aggregate_type NOT NULL,

    parent_id       UUID,
    parent_type     aggregate_type,

    event_type      VARCHAR(128) NOT NULL,
    payload         JSONB NOT NULL,

    occurred_at     TIMESTAMPTZ NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    processed_at    TIMESTAMPTZ,
    last_attempt_at TIMESTAMPTZ,

    attempts        SMALLINT NOT NULL DEFAULT 0,
    last_error      TEXT,

    CONSTRAINT outbox_parent_consistency CHECK (
        (parent_id IS NULL AND parent_type IS NULL)
        OR (parent_id IS NOT NULL AND parent_type IS NOT NULL)
    ),
    CONSTRAINT outbox_attempts_check CHECK (attempts >= 0 AND attempts <= 100)
);

CREATE INDEX outbox_pending_idx
    ON outbox (last_attempt_at NULLS FIRST, created_at)
    WHERE processed_at IS NULL;

CREATE INDEX outbox_aggregate_idx
    ON outbox (aggregate_type, aggregate_id, created_at);

CREATE INDEX outbox_processed_idx
    ON outbox (processed_at)
    WHERE processed_at IS NOT NULL;
