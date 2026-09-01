CREATE TABLE messaging.user_privacy_settings (
    user_id                UUID PRIMARY KEY,
    dm_policy              VARCHAR(10) NOT NULL DEFAULT 'friends',
    accept_friend_requests BOOLEAN NOT NULL DEFAULT true,
    updated_at             TIMESTAMPTZ NOT NULL DEFAULT now(),

    CONSTRAINT user_privacy_dm_policy_check CHECK (dm_policy IN ('all', 'friends', 'no_one'))
);
