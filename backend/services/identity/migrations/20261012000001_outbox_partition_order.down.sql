DROP INDEX identity.outbox_partition_pending_idx;
DROP INDEX identity.outbox_pending_idx;
CREATE INDEX outbox_pending_idx ON identity.outbox (last_attempt_at NULLS FIRST, created_at) WHERE (processed_at IS NULL);

ALTER TABLE identity.outbox
    DROP COLUMN seq,
    DROP COLUMN partition_key;
