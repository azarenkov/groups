CREATE TABLE messaging.search_history (
    id uuid NOT NULL,
    user_id uuid NOT NULL,
    kind character varying(16) NOT NULL,
    key text NOT NULL,
    query text,
    target_id uuid,
    used_at timestamp with time zone NOT NULL,
    CONSTRAINT search_history_pkey PRIMARY KEY (id),
    CONSTRAINT search_history_entry_key UNIQUE (user_id, kind, key),
    CONSTRAINT search_history_kind_check CHECK (kind IN ('query', 'person', 'dm', 'community', 'channel', 'message')),
    CONSTRAINT search_history_target_check CHECK (
        (kind = 'query' AND query IS NOT NULL AND target_id IS NULL)
        OR (kind <> 'query' AND query IS NULL AND target_id IS NOT NULL)
    )
);

CREATE INDEX search_history_recent_idx ON messaging.search_history (user_id, used_at DESC);
