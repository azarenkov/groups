DROP TABLE IF EXISTS identity.trial_grants;
DROP TABLE IF EXISTS identity.user_entitlements;
DROP SEQUENCE IF EXISTS identity.pro_member_number_seq;
DROP TABLE IF EXISTS identity.oauth_identities;
DROP TABLE IF EXISTS identity.connection_states;

ALTER TABLE identity.users
    DROP CONSTRAINT IF EXISTS users_badge_style_check,
    DROP COLUMN IF EXISTS badge_style;
