DROP TRIGGER IF EXISTS dm_conversations_sync ON messaging.dm_messages;
DROP FUNCTION IF EXISTS messaging.dm_conversations_sync();
DROP TABLE IF EXISTS messaging.dm_conversations;
