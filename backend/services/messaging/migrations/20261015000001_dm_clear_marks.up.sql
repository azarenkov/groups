CREATE TABLE messaging.dm_clear_marks (
    user_id uuid NOT NULL,
    peer_id uuid NOT NULL,
    cleared_at timestamp with time zone NOT NULL,
    CONSTRAINT dm_clear_marks_pkey PRIMARY KEY (user_id, peer_id)
);
