CREATE TABLE messaging.community_invitations (
    id uuid NOT NULL,
    community_id uuid NOT NULL,
    inviter_id uuid NOT NULL,
    invitee_id uuid NOT NULL,
    created_at timestamp with time zone NOT NULL,
    CONSTRAINT community_invitations_pkey PRIMARY KEY (id),
    CONSTRAINT community_invitations_pending_key UNIQUE (community_id, invitee_id),
    CONSTRAINT community_invitations_community_fkey FOREIGN KEY (community_id)
        REFERENCES messaging.communities (id) ON DELETE CASCADE,
    CONSTRAINT community_invitations_check CHECK (inviter_id <> invitee_id)
);

CREATE INDEX community_invitations_invitee_idx ON messaging.community_invitations (invitee_id);

CREATE TABLE messaging.inbox_notifications (
    id uuid NOT NULL,
    user_id uuid NOT NULL,
    kind character varying(32) NOT NULL,
    actor_id uuid NOT NULL,
    community_id uuid,
    community_name text,
    invitation_id uuid,
    source_event_id text NOT NULL,
    created_at timestamp with time zone NOT NULL,
    read_at timestamp with time zone,
    CONSTRAINT inbox_notifications_pkey PRIMARY KEY (id),
    CONSTRAINT inbox_notifications_source_key UNIQUE (user_id, source_event_id),
    CONSTRAINT inbox_notifications_kind_check CHECK (
        kind IN ('community_invitation', 'removed_from_community', 'friend_request_received', 'friend_request_accepted')
    ),
    CONSTRAINT inbox_notifications_invitation_fkey FOREIGN KEY (invitation_id)
        REFERENCES messaging.community_invitations (id) ON DELETE CASCADE
);

CREATE INDEX inbox_notifications_recent_idx ON messaging.inbox_notifications (user_id, created_at DESC, id DESC);
CREATE INDEX inbox_notifications_unread_idx ON messaging.inbox_notifications (user_id) WHERE read_at IS NULL;

INSERT INTO messaging.community_invitations (id, community_id, inviter_id, invitee_id, created_at)
SELECT DISTINCT ON (m.invite_community_id, m.recipient_id)
       gen_random_uuid(), m.invite_community_id, m.sender_id, m.recipient_id, m.created_at
FROM messaging.dm_messages m
WHERE m.invite_community_id IS NOT NULL
  AND m.sender_id <> m.recipient_id
  AND NOT EXISTS (
      SELECT 1 FROM messaging.community_members cm
      WHERE cm.community_id = m.invite_community_id AND cm.user_id = m.recipient_id
  )
ORDER BY m.invite_community_id, m.recipient_id, m.created_at DESC;

INSERT INTO messaging.inbox_notifications (id, user_id, kind, actor_id, community_id, invitation_id, source_event_id, created_at)
SELECT gen_random_uuid(), i.invitee_id, 'community_invitation', i.inviter_id, i.community_id, i.id, 'migrated:' || i.id, i.created_at
FROM messaging.community_invitations i;

DELETE FROM messaging.dm_messages WHERE invite_community_id IS NOT NULL;
