CREATE TABLE dm_messages (
    id           UUID PRIMARY KEY,
    sender_id    UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    recipient_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    content      TEXT NOT NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    edited_at    TIMESTAMPTZ,
    search_tsv   tsvector GENERATED ALWAYS AS (to_tsvector('simple', content)) STORED,

    CHECK (sender_id <> recipient_id)
);

CREATE INDEX idx_dm_messages_pair_created
    ON dm_messages (LEAST(sender_id, recipient_id), GREATEST(sender_id, recipient_id), created_at DESC);

CREATE INDEX idx_dm_messages_search
    ON dm_messages USING GIN (search_tsv);
