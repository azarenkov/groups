ALTER TABLE messaging.messages DROP COLUMN IF EXISTS system_payload;
ALTER TABLE messaging.messages DROP CONSTRAINT IF EXISTS messages_kind_check;
ALTER TABLE messaging.messages DROP COLUMN IF EXISTS kind;

DROP INDEX IF EXISTS messaging.uq_channels_default_per_server;
ALTER TABLE messaging.channels DROP COLUMN IF EXISTS is_default;
