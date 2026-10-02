DELETE FROM messaging.reports WHERE target_type = 'sticker_pack';
ALTER TABLE messaging.reports DROP CONSTRAINT reports_target_type_check;
ALTER TABLE messaging.reports ADD CONSTRAINT reports_target_type_check CHECK (target_type IN ('user', 'community'));
ALTER TABLE messaging.reports ALTER COLUMN target_type TYPE varchar(10);
