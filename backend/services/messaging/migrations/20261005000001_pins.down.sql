DROP TRIGGER IF EXISTS conversation_pins_member_cleanup ON messaging.community_members;
DROP FUNCTION IF EXISTS messaging.conversation_pins_member_cleanup();
DROP TABLE IF EXISTS messaging.conversation_pins;
DROP TABLE IF EXISTS messaging.dm_pinned_messages;
DROP TABLE IF EXISTS messaging.channel_pinned_messages;
