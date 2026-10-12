ALTER TABLE identity.outbox
    ADD COLUMN partition_key uuid,
    ADD COLUMN seq bigint GENERATED ALWAYS AS IDENTITY;

DROP INDEX identity.outbox_pending_idx;
CREATE INDEX outbox_pending_idx ON identity.outbox (last_attempt_at NULLS FIRST, seq) WHERE (processed_at IS NULL);
CREATE INDEX outbox_partition_pending_idx ON identity.outbox (partition_key, seq) WHERE (processed_at IS NULL);
