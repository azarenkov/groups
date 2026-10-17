DROP INDEX IF EXISTS messaging.idx_channels_last_join;

ALTER TABLE messaging.channels
    DROP COLUMN IF EXISTS last_join;

CREATE INDEX idx_call_log_consumed_created_at
    ON messaging.call_log_consumed (created_at);
