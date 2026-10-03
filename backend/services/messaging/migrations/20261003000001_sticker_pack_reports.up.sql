ALTER TABLE messaging.reports ALTER COLUMN target_type TYPE varchar(20);
ALTER TABLE messaging.reports DROP CONSTRAINT reports_target_type_check;
ALTER TABLE messaging.reports ADD CONSTRAINT reports_target_type_check CHECK (target_type IN ('user', 'community', 'sticker_pack'));
