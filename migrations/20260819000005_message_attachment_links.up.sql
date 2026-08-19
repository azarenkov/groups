CREATE TABLE message_attachments (
    message_id    UUID NOT NULL,
    attachment_id UUID NOT NULL,
    file_name     TEXT NOT NULL,
    content_type  TEXT NOT NULL,
    size_bytes    BIGINT NOT NULL,
    storage_key   TEXT NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (message_id, attachment_id)
);

CREATE TABLE dm_message_attachments (
    dm_message_id UUID NOT NULL,
    attachment_id UUID NOT NULL,
    file_name     TEXT NOT NULL,
    content_type  TEXT NOT NULL,
    size_bytes    BIGINT NOT NULL,
    storage_key   TEXT NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (dm_message_id, attachment_id)
);

ALTER TABLE attachments ADD COLUMN claimed_at TIMESTAMPTZ;

UPDATE attachments SET claimed_at = created_at
WHERE message_id IS NOT NULL OR dm_message_id IS NOT NULL;

INSERT INTO message_attachments
    (message_id, attachment_id, file_name, content_type, size_bytes, storage_key, created_at)
SELECT message_id, id, file_name, content_type, size_bytes, storage_key, created_at
FROM attachments WHERE message_id IS NOT NULL;

INSERT INTO dm_message_attachments
    (dm_message_id, attachment_id, file_name, content_type, size_bytes, storage_key, created_at)
SELECT dm_message_id, id, file_name, content_type, size_bytes, storage_key, created_at
FROM attachments WHERE dm_message_id IS NOT NULL;

ALTER TABLE attachments DROP COLUMN message_id;
ALTER TABLE attachments DROP COLUMN dm_message_id;

CREATE INDEX idx_attachments_unclaimed ON attachments (created_at)
    WHERE claimed_at IS NULL AND deleting_at IS NULL;
