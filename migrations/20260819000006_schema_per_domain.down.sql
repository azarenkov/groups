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


ALTER TABLE attachments ADD CONSTRAINT attachments_owner_id_fkey
    FOREIGN KEY (owner_id) REFERENCES users(id) ON DELETE CASCADE;
ALTER TABLE broadcasts ADD CONSTRAINT broadcasts_dm_user_a_fkey
    FOREIGN KEY (dm_user_a) REFERENCES users(id) ON DELETE CASCADE;
ALTER TABLE broadcasts ADD CONSTRAINT broadcasts_dm_user_b_fkey
    FOREIGN KEY (dm_user_b) REFERENCES users(id) ON DELETE CASCADE;
ALTER TABLE broadcasts ADD CONSTRAINT broadcasts_broadcaster_id_fkey
    FOREIGN KEY (broadcaster_id) REFERENCES users(id) ON DELETE CASCADE;
ALTER TABLE call_participants ADD CONSTRAINT call_participants_inviter_id_fkey
    FOREIGN KEY (inviter_id) REFERENCES users(id) ON DELETE CASCADE;
ALTER TABLE call_participants ADD CONSTRAINT call_participants_user_id_fkey
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;
ALTER TABLE channel_muted_participants ADD CONSTRAINT channel_muted_participants_muted_by_fkey
    FOREIGN KEY (muted_by) REFERENCES users(id) ON DELETE RESTRICT;
ALTER TABLE channel_muted_participants ADD CONSTRAINT channel_muted_participants_user_id_fkey
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;
ALTER TABLE channel_read_state ADD CONSTRAINT channel_read_state_user_id_fkey
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;
ALTER TABLE channel_stream_viewers ADD CONSTRAINT channel_stream_viewers_viewer_id_fkey
    FOREIGN KEY (viewer_id) REFERENCES users(id) ON DELETE CASCADE;
ALTER TABLE channel_stream_viewers ADD CONSTRAINT channel_stream_viewers_streamer_id_fkey
    FOREIGN KEY (streamer_id) REFERENCES users(id) ON DELETE CASCADE;
ALTER TABLE channel_text_participants ADD CONSTRAINT channel_text_participants_user_id_fkey
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;
ALTER TABLE channel_voice_participants ADD CONSTRAINT channel_voice_participants_user_id_fkey
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;
ALTER TABLE channels ADD CONSTRAINT channels_owner_id_fkey
    FOREIGN KEY (owner_id) REFERENCES users(id) ON DELETE CASCADE;
ALTER TABLE dm_messages ADD CONSTRAINT dm_messages_recipient_id_fkey
    FOREIGN KEY (recipient_id) REFERENCES users(id) ON DELETE CASCADE;
ALTER TABLE dm_messages ADD CONSTRAINT dm_messages_sender_id_fkey
    FOREIGN KEY (sender_id) REFERENCES users(id) ON DELETE CASCADE;
ALTER TABLE dm_read_state ADD CONSTRAINT dm_read_state_user_id_fkey
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;
ALTER TABLE dm_read_state ADD CONSTRAINT dm_read_state_friend_id_fkey
    FOREIGN KEY (friend_id) REFERENCES users(id) ON DELETE CASCADE;
ALTER TABLE friendships ADD CONSTRAINT friendships_user_id_fkey
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;
ALTER TABLE friendships ADD CONSTRAINT friendships_friend_id_fkey
    FOREIGN KEY (friend_id) REFERENCES users(id) ON DELETE CASCADE;
ALTER TABLE messages ADD CONSTRAINT messages_author_id_fkey
    FOREIGN KEY (author_id) REFERENCES users(id) ON DELETE CASCADE;
ALTER TABLE server_invite_codes ADD CONSTRAINT server_invite_codes_created_by_fkey
    FOREIGN KEY (created_by) REFERENCES users(id) ON DELETE CASCADE;
ALTER TABLE server_members ADD CONSTRAINT server_members_user_id_fkey
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;
ALTER TABLE servers ADD CONSTRAINT servers_owner_id_fkey
    FOREIGN KEY (owner_id) REFERENCES users(id) ON DELETE CASCADE;

DO $$
BEGIN
    EXECUTE format('ALTER DATABASE %I SET search_path TO public', current_database());
END $$;

DROP SCHEMA IF EXISTS identity;
DROP SCHEMA IF EXISTS media;
DROP SCHEMA IF EXISTS messaging;
DROP SCHEMA IF EXISTS voice;
DROP SCHEMA IF EXISTS realtime;
