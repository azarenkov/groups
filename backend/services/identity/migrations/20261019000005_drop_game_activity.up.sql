DROP SEQUENCE IF EXISTS identity.activity_versions;

ALTER TABLE identity.users
    DROP COLUMN IF EXISTS show_game_activity;
