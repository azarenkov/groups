ALTER TABLE messaging.outbox DROP COLUMN partition_seq;

DROP TABLE messaging.outbox_partition_seq;
