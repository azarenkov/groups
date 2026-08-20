CREATE TABLE messaging_user_profiles (
    user_id UUID PRIMARY KEY,
    name    TEXT NOT NULL
);

CREATE TABLE voice_user_profiles (
    user_id UUID PRIMARY KEY,
    name    TEXT NOT NULL
);

INSERT INTO messaging_user_profiles (user_id, name)
SELECT id, name FROM users;

INSERT INTO voice_user_profiles (user_id, name)
SELECT id, name FROM users;
