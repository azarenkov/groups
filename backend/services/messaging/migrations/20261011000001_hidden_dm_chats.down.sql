DROP TRIGGER IF EXISTS hidden_dm_chats_deleted_user_cleanup ON messaging.messaging_deleted_users;
DROP FUNCTION IF EXISTS messaging.hidden_dm_chats_deleted_user_cleanup();
DROP TABLE IF EXISTS messaging.hidden_dm_chats;
