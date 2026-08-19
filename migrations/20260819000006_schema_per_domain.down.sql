ALTER TABLE identity.users SET SCHEMA public;
ALTER TABLE identity.refresh_tokens SET SCHEMA public;
ALTER TABLE identity.verification_codes SET SCHEMA public;
ALTER TABLE identity.oauth_identities SET SCHEMA public;
ALTER TABLE media.attachments SET SCHEMA public;
ALTER TABLE media.media_user_avatars SET SCHEMA public;
ALTER TABLE media.media_server_logos SET SCHEMA public;
ALTER TABLE media.media_server_owners SET SCHEMA public;
ALTER TABLE messaging.servers SET SCHEMA public;
ALTER TABLE messaging.server_members SET SCHEMA public;
ALTER TABLE messaging.server_invite_codes SET SCHEMA public;
ALTER TABLE messaging.channels SET SCHEMA public;
ALTER TABLE messaging.channel_muted_participants SET SCHEMA public;
ALTER TABLE messaging.channel_read_state SET SCHEMA public;
ALTER TABLE messaging.channel_stream_viewers SET SCHEMA public;
ALTER TABLE messaging.channel_text_participants SET SCHEMA public;
ALTER TABLE messaging.channel_voice_participants SET SCHEMA public;
ALTER TABLE messaging.messages SET SCHEMA public;
ALTER TABLE messaging.message_attachments SET SCHEMA public;
ALTER TABLE messaging.dm_messages SET SCHEMA public;
ALTER TABLE messaging.dm_message_attachments SET SCHEMA public;
ALTER TABLE messaging.dm_read_state SET SCHEMA public;
ALTER TABLE messaging.friendships SET SCHEMA public;
ALTER TABLE messaging.messaging_user_profiles SET SCHEMA public;
ALTER TABLE messaging.broadcasts SET SCHEMA public;
ALTER TABLE messaging.webhook_inbox SET SCHEMA public;
ALTER TABLE voice.call_participants SET SCHEMA public;
ALTER TABLE voice.voice_user_profiles SET SCHEMA public;
ALTER TABLE voice.voice_friend_edges SET SCHEMA public;
ALTER TABLE realtime.realtime_friend_edges SET SCHEMA public;
ALTER TABLE realtime.realtime_server_members SET SCHEMA public;
ALTER TABLE realtime.realtime_user_names SET SCHEMA public;

DO $$
BEGIN
    EXECUTE format('ALTER DATABASE %I SET search_path TO public', current_database());
END $$;

DROP SCHEMA IF EXISTS identity;
DROP SCHEMA IF EXISTS media;
DROP SCHEMA IF EXISTS messaging;
DROP SCHEMA IF EXISTS voice;
DROP SCHEMA IF EXISTS realtime;
