DROP INDEX IF EXISTS idx_users_name;

DO $$
DECLARE
    loser RECORD;
    base TEXT;
    suffix TEXT;
    candidate TEXT;
    attempt INT;
BEGIN
    FOR loser IN
        SELECT u.id, u.name FROM users u
        WHERE EXISTS (
            SELECT 1 FROM users o
            WHERE LOWER(o.name COLLATE "C") = LOWER(u.name COLLATE "C") AND o.id <> u.id
        )
        AND u.id NOT IN (
            SELECT DISTINCT ON (LOWER(name COLLATE "C")) id
            FROM users
            ORDER BY LOWER(name COLLATE "C"), created_at, id
        )
        ORDER BY u.id
    LOOP
        base := btrim(regexp_replace(left(loser.name, 8), '_+', '_', 'g'), '_');
        IF base = '' THEN
            base := 'user';
        END IF;
        attempt := 0;
        LOOP
            IF attempt = 0 THEN
                suffix := substr(replace(loser.id::text, '-', ''), 1, 6);
            ELSE
                suffix := substr(md5(loser.id::text || attempt::text), 1, 6);
            END IF;
            candidate := base || '_' || suffix;
            EXIT WHEN NOT EXISTS (
                SELECT 1 FROM users WHERE LOWER(name COLLATE "C") = LOWER(candidate COLLATE "C")
            );
            attempt := attempt + 1;
        END LOOP;
        UPDATE users SET name = candidate WHERE id = loser.id;
    END LOOP;
END $$;

CREATE UNIQUE INDEX users_name_lower_key ON users (LOWER(name COLLATE "C"));
