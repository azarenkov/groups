DROP INDEX IF EXISTS outbox_pending_idx;

CREATE INDEX outbox_unprocessed_idx
    ON outbox (created_at)
    WHERE processed_at IS NULL;

ALTER TABLE outbox
    DROP COLUMN IF EXISTS last_attempt_at;
