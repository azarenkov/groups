DROP INDEX IF EXISTS idx_users_name;

UPDATE users u
SET name = left(u.name, 40) || '_' || substr(replace(u.id::text, '-', ''), 1, 6)
WHERE EXISTS (
    SELECT 1 FROM users o
    WHERE LOWER(o.name) = LOWER(u.name) AND o.id <> u.id
)
AND u.id NOT IN (
    SELECT DISTINCT ON (LOWER(name)) id
    FROM users
    ORDER BY LOWER(name), created_at, id
);

CREATE UNIQUE INDEX users_name_lower_key ON users (LOWER(name));
