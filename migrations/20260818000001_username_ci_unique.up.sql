DROP INDEX IF EXISTS idx_users_name;

DO $$
DECLARE
    loser RECORD;
    candidate TEXT;
    attempt INT;
BEGIN
    FOR loser IN
        SELECT u.id, u.name FROM users u
        WHERE EXISTS (
            SELECT 1 FROM users o
            WHERE LOWER(o.name) = LOWER(u.name) AND o.id <> u.id
        )
        AND u.id NOT IN (
            SELECT DISTINCT ON (LOWER(name)) id
            FROM users
            ORDER BY LOWER(name), created_at, id
        )
        ORDER BY u.id
    LOOP
        attempt := 0;
        LOOP
            candidate := left(loser.name, 40) || '_'
                || substr(replace(loser.id::text, '-', ''), 1, 6)
                || CASE WHEN attempt = 0 THEN '' ELSE '_' || attempt::text END;
            EXIT WHEN NOT EXISTS (
                SELECT 1 FROM users WHERE LOWER(name) = LOWER(candidate)
            );
            attempt := attempt + 1;
        END LOOP;
        UPDATE users SET name = candidate WHERE id = loser.id;
    END LOOP;
END $$;

CREATE UNIQUE INDEX users_name_lower_key ON users (LOWER(name));
