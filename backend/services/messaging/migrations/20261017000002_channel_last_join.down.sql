DROP INDEX IF EXISTS messaging.idx_call_log_consumed_created_at;

ALTER TABLE messaging.channels
    ADD COLUMN last_join timestamp with time zone DEFAULT now() NOT NULL;

CREATE INDEX idx_channels_last_join ON messaging.channels (last_join);
