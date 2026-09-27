DROP TRIGGER hidden_messages_cleanup ON messaging.dm_messages;
DROP TRIGGER hidden_messages_cleanup ON messaging.messages;
DROP FUNCTION messaging.hidden_messages_cleanup();
DROP TABLE messaging.hidden_messages;
