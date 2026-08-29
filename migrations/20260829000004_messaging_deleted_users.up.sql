CREATE TABLE messaging.messaging_deleted_users (
    user_id    UUID PRIMARY KEY,
    deleted_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

INSERT INTO messaging.messaging_deleted_users (user_id, deleted_at)
SELECT id, deleted_at FROM identity.users WHERE deleted_at IS NOT NULL
ON CONFLICT DO NOTHING;
