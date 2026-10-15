CREATE TABLE messaging.inbox_feed_start (
    singleton boolean NOT NULL DEFAULT true,
    started_at timestamp with time zone NOT NULL,
    CONSTRAINT inbox_feed_start_pkey PRIMARY KEY (singleton),
    CONSTRAINT inbox_feed_start_singleton_check CHECK (singleton)
);

INSERT INTO messaging.inbox_feed_start (singleton, started_at) VALUES (true, date_trunc('milliseconds', now()));
