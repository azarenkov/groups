CREATE TABLE messaging.feed_starts (
    feed character varying(32) NOT NULL,
    started_at timestamp with time zone NOT NULL,
    CONSTRAINT feed_starts_pkey PRIMARY KEY (feed)
);

INSERT INTO messaging.feed_starts (feed, started_at)
SELECT 'inbox', started_at FROM messaging.inbox_feed_start;

INSERT INTO messaging.feed_starts (feed, started_at)
VALUES ('call_log', date_trunc('milliseconds', now()));
