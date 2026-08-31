ALTER TABLE messaging.user_privacy_settings
    ADD COLUMN call_policy VARCHAR(10) NOT NULL DEFAULT 'friends',
    ADD CONSTRAINT user_privacy_call_policy_check CHECK (call_policy IN ('all', 'friends', 'no_one'));
