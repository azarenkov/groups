ALTER TABLE identity.users
    ADD COLUMN show_game_activity boolean DEFAULT true NOT NULL;

CREATE SEQUENCE identity.activity_versions;
