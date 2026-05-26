ALTER TABLE users
ADD COLUMN IF NOT EXISTS locale TEXT NOT NULL DEFAULT 'en';

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM information_schema.table_constraints
        WHERE table_name = 'users' AND constraint_name = 'users_locale_check'
    ) THEN
        ALTER TABLE users ADD CONSTRAINT users_locale_check CHECK (locale IN ('en', 'ru'));
    END IF;
END$$;
