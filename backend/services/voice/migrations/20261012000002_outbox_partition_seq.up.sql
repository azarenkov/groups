CREATE TABLE voice.outbox_partition_seq (
    partition_key uuid NOT NULL,
    seq bigint NOT NULL,
    CONSTRAINT outbox_partition_seq_pkey PRIMARY KEY (partition_key)
);

ALTER TABLE voice.outbox ADD COLUMN partition_seq bigint;
