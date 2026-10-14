DROP INDEX messaging.outbox_partition_pending_idx;
DROP INDEX messaging.outbox_pending_idx;
CREATE INDEX outbox_pending_idx ON messaging.outbox (last_attempt_at NULLS FIRST, created_at) WHERE (processed_at IS NULL);

ALTER TABLE messaging.outbox
    DROP COLUMN seq,
    DROP COLUMN partition_key;
