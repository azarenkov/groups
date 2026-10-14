DELETE FROM messaging.community_invitations i
USING messaging.user_blocks b
WHERE (b.blocker_id = i.inviter_id AND b.blocked_id = i.invitee_id)
   OR (b.blocker_id = i.invitee_id AND b.blocked_id = i.inviter_id);
