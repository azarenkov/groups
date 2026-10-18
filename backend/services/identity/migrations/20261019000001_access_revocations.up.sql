CREATE TABLE identity.access_revocations (
    user_id uuid NOT NULL,
    cutoff timestamp with time zone NOT NULL,
    recorded_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT access_revocations_pkey PRIMARY KEY (user_id)
);
