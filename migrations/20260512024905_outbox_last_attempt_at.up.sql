ALTER TABLE outbox
    ADD COLUMN last_attempt_at TIMESTAMPTZ;

DROP INDEX IF EXISTS outbox_unprocessed_idx;

CREATE INDEX outbox_pending_idx
    ON outbox (last_attempt_at NULLS FIRST, created_at)
    WHERE processed_at IS NULL;
