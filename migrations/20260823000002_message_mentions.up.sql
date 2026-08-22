ALTER TABLE messaging.messages ADD COLUMN mentions UUID[] NOT NULL DEFAULT '{}';
