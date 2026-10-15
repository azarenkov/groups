CREATE TABLE messaging.removed_legacy_invitation_messages (
    id uuid NOT NULL,
    sender_id uuid NOT NULL,
    recipient_id uuid NOT NULL,
    CONSTRAINT removed_legacy_invitation_messages_pkey PRIMARY KEY (id)
);

INSERT INTO messaging.removed_legacy_invitation_messages (id, sender_id, recipient_id)
SELECT id, sender_id, recipient_id FROM messaging.dm_messages WHERE invite_community_id IS NOT NULL;

DELETE FROM messaging.dm_messages m
USING messaging.user_blocks b
WHERE m.invite_community_id IS NOT NULL
  AND ((b.blocker_id = m.sender_id AND b.blocked_id = m.recipient_id)
    OR (b.blocker_id = m.recipient_id AND b.blocked_id = m.sender_id));

DELETE FROM messaging.dm_messages m
WHERE m.invite_community_id IS NOT NULL
  AND EXISTS (
      SELECT 1 FROM messaging.messaging_deleted_users d
      WHERE d.user_id = m.sender_id OR d.user_id = m.recipient_id
  );
