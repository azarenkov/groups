ALTER TABLE identity.users
    ADD COLUMN role text NOT NULL DEFAULT 'student';

UPDATE identity.users SET role = 'teacher';

ALTER TABLE identity.users
    ADD CONSTRAINT users_role_check CHECK (role IN ('student', 'teacher'));
