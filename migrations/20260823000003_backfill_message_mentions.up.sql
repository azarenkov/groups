UPDATE messaging.messages m
SET mentions = sub.ids
FROM (
    SELECT m2.id, array_agg(DISTINCT p.user_id) AS ids
    FROM messaging.messages m2
    INNER JOIN messaging.channel_text_participants p ON p.channel_id = m2.channel_id
    INNER JOIN messaging.messaging_user_profiles up ON up.user_id = p.user_id
    WHERE m2.mentions = '{}'
      AND m2.content LIKE '%@%'
      AND m2.content ~* ('(^|[^a-z0-9_@])@' || up.name || '([^a-z0-9_]|$)')
    GROUP BY m2.id
) sub
WHERE m.id = sub.id;
