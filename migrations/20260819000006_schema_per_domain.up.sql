CREATE SCHEMA IF NOT EXISTS identity;
CREATE SCHEMA IF NOT EXISTS media;
CREATE SCHEMA IF NOT EXISTS messaging;
CREATE SCHEMA IF NOT EXISTS voice;
CREATE SCHEMA IF NOT EXISTS realtime;

ALTER TABLE attachments DROP CONSTRAINT IF EXISTS attachments_owner_id_fkey;
ALTER TABLE broadcasts DROP CONSTRAINT IF EXISTS broadcasts_dm_user_a_fkey;
ALTER TABLE broadcasts DROP CONSTRAINT IF EXISTS broadcasts_dm_user_b_fkey;
ALTER TABLE broadcasts DROP CONSTRAINT IF EXISTS broadcasts_broadcaster_id_fkey;
ALTER TABLE call_participants DROP CONSTRAINT IF EXISTS call_participants_inviter_id_fkey;
ALTER TABLE call_participants DROP CONSTRAINT IF EXISTS call_participants_user_id_fkey;
ALTER TABLE channel_muted_participants DROP CONSTRAINT IF EXISTS channel_muted_participants_muted_by_fkey;
ALTER TABLE channel_muted_participants DROP CONSTRAINT IF EXISTS channel_muted_participants_user_id_fkey;
ALTER TABLE channel_read_state DROP CONSTRAINT IF EXISTS channel_read_state_user_id_fkey;
ALTER TABLE channel_stream_viewers DROP CONSTRAINT IF EXISTS channel_stream_viewers_viewer_id_fkey;
ALTER TABLE channel_stream_viewers DROP CONSTRAINT IF EXISTS channel_stream_viewers_streamer_id_fkey;
ALTER TABLE channel_text_participants DROP CONSTRAINT IF EXISTS channel_text_participants_user_id_fkey;
ALTER TABLE channel_voice_participants DROP CONSTRAINT IF EXISTS channel_voice_participants_user_id_fkey;
ALTER TABLE channels DROP CONSTRAINT IF EXISTS channels_owner_id_fkey;
ALTER TABLE dm_messages DROP CONSTRAINT IF EXISTS dm_messages_recipient_id_fkey;
ALTER TABLE dm_messages DROP CONSTRAINT IF EXISTS dm_messages_sender_id_fkey;
ALTER TABLE dm_read_state DROP CONSTRAINT IF EXISTS dm_read_state_user_id_fkey;
ALTER TABLE dm_read_state DROP CONSTRAINT IF EXISTS dm_read_state_friend_id_fkey;
ALTER TABLE friendships DROP CONSTRAINT IF EXISTS friendships_user_id_fkey;
ALTER TABLE friendships DROP CONSTRAINT IF EXISTS friendships_friend_id_fkey;
ALTER TABLE messages DROP CONSTRAINT IF EXISTS messages_author_id_fkey;
ALTER TABLE server_invite_codes DROP CONSTRAINT IF EXISTS server_invite_codes_created_by_fkey;
ALTER TABLE server_members DROP CONSTRAINT IF EXISTS server_members_user_id_fkey;
ALTER TABLE servers DROP CONSTRAINT IF EXISTS servers_owner_id_fkey;

ALTER TABLE users SET SCHEMA identity;
ALTER TABLE refresh_tokens SET SCHEMA identity;
ALTER TABLE verification_codes SET SCHEMA identity;
ALTER TABLE oauth_identities SET SCHEMA identity;

ALTER TABLE attachments SET SCHEMA media;
ALTER TABLE media_user_avatars SET SCHEMA media;
ALTER TABLE media_server_logos SET SCHEMA media;
ALTER TABLE media_server_owners SET SCHEMA media;

ALTER TABLE servers SET SCHEMA messaging;
ALTER TABLE server_members SET SCHEMA messaging;
ALTER TABLE server_invite_codes SET SCHEMA messaging;
ALTER TABLE channels SET SCHEMA messaging;
ALTER TABLE channel_muted_participants SET SCHEMA messaging;
ALTER TABLE channel_read_state SET SCHEMA messaging;
ALTER TABLE channel_stream_viewers SET SCHEMA messaging;
ALTER TABLE channel_text_participants SET SCHEMA messaging;
ALTER TABLE channel_voice_participants SET SCHEMA messaging;
ALTER TABLE messages SET SCHEMA messaging;
ALTER TABLE message_attachments SET SCHEMA messaging;
ALTER TABLE dm_messages SET SCHEMA messaging;
ALTER TABLE dm_message_attachments SET SCHEMA messaging;
ALTER TABLE dm_read_state SET SCHEMA messaging;
ALTER TABLE friendships SET SCHEMA messaging;
ALTER TABLE messaging_user_profiles SET SCHEMA messaging;
ALTER TABLE broadcasts SET SCHEMA messaging;
ALTER TABLE webhook_inbox SET SCHEMA messaging;

ALTER TABLE call_participants SET SCHEMA voice;
ALTER TABLE voice_user_profiles SET SCHEMA voice;
ALTER TABLE voice_friend_edges SET SCHEMA voice;

ALTER TABLE realtime_friend_edges SET SCHEMA realtime;
ALTER TABLE realtime_server_members SET SCHEMA realtime;
ALTER TABLE realtime_user_names SET SCHEMA realtime;

DO $$
BEGIN
    EXECUTE format(
        'ALTER DATABASE %I SET search_path TO public, identity, media, messaging, voice, realtime',
        current_database()
    );
END $$;
