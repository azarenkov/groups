DROP TRIGGER voice_session_end ON messaging.channel_voice_participants;
DROP TRIGGER voice_session_start ON messaging.channel_voice_participants;
DROP FUNCTION messaging.voice_session_end();
DROP FUNCTION messaging.voice_session_start();

ALTER TABLE messaging.channels
    DROP COLUMN voice_session_started_at;
