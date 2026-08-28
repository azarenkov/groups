CREATE TABLE media.media_deleted_users (
    user_id    UUID PRIMARY KEY,
    deleted_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

INSERT INTO media.media_deleted_users (user_id, deleted_at)
SELECT id, deleted_at FROM identity.users WHERE deleted_at IS NOT NULL
ON CONFLICT DO NOTHING;

DELETE FROM media.media_user_avatars a
USING media.media_deleted_users d
WHERE d.user_id = a.user_id;
