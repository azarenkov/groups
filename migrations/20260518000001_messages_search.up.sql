ALTER TABLE messages
    ADD COLUMN search_tsv tsvector
        GENERATED ALWAYS AS (to_tsvector('simple', content)) STORED;

CREATE INDEX idx_messages_search_tsv
    ON messages USING GIN (search_tsv);
