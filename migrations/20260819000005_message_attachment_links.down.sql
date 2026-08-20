ALTER TABLE attachments ADD COLUMN message_id UUID;
ALTER TABLE attachments ADD COLUMN dm_message_id UUID;

UPDATE attachments a SET message_id = l.message_id
FROM message_attachments l WHERE l.attachment_id = a.id;

UPDATE attachments a SET dm_message_id = l.dm_message_id
FROM dm_message_attachments l WHERE l.attachment_id = a.id;

ALTER TABLE attachments DROP COLUMN claimed_at;

DROP TABLE message_attachments;
DROP TABLE dm_message_attachments;
