DROP INDEX voice.outbox_partition_pending_idx;
DROP INDEX voice.outbox_pending_idx;
CREATE INDEX outbox_pending_idx ON voice.outbox (last_attempt_at NULLS FIRST, created_at) WHERE (processed_at IS NULL);

ALTER TABLE voice.outbox
    DROP COLUMN seq,
    DROP COLUMN partition_key;
