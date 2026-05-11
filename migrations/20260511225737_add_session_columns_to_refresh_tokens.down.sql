DROP INDEX IF EXISTS refresh_tokens_user_id_last_used_idx;

ALTER TABLE refresh_tokens
    DROP COLUMN IF EXISTS last_used_at,
    DROP COLUMN IF EXISTS last_user_agent,
    DROP COLUMN IF EXISTS last_ip;
