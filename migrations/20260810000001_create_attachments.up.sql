CREATE TABLE attachments (
    id UUID PRIMARY KEY,
    owner_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    file_name TEXT NOT NULL,
    content_type TEXT NOT NULL,
    size_bytes BIGINT NOT NULL,
    storage_key TEXT NOT NULL,
    message_id UUID REFERENCES messages(id) ON DELETE CASCADE,
    dm_message_id UUID REFERENCES dm_messages(id) ON DELETE CASCADE,
    deleting_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_attachments_message ON attachments (message_id) WHERE message_id IS NOT NULL;
CREATE INDEX idx_attachments_dm_message ON attachments (dm_message_id) WHERE dm_message_id IS NOT NULL;
CREATE INDEX idx_attachments_owner_unlinked ON attachments (owner_id, created_at)
    WHERE message_id IS NULL AND dm_message_id IS NULL;
