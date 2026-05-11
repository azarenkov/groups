CREATE TYPE webhook_source AS ENUM ('livekit');

CREATE TABLE webhook_inbox (
    source       webhook_source NOT NULL,
    external_id  TEXT NOT NULL,
    processed_at TIMESTAMPTZ NOT NULL DEFAULT now(),

    PRIMARY KEY (source, external_id)
);

CREATE INDEX webhook_inbox_processed_at_idx ON webhook_inbox (processed_at);
