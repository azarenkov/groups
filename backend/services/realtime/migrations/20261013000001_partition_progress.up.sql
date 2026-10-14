CREATE TABLE realtime.realtime_partition_progress (
    partition text NOT NULL,
    seq bigint NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT realtime_partition_progress_pkey PRIMARY KEY (partition)
);
