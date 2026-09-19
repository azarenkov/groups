CREATE SEQUENCE identity.pro_member_number_seq;

ALTER TABLE identity.user_entitlements
    ADD COLUMN member_number bigint;

CREATE UNIQUE INDEX user_entitlements_member_number_idx
    ON identity.user_entitlements (member_number)
    WHERE member_number IS NOT NULL;

WITH ordered AS (
    SELECT user_id, row_number() OVER (ORDER BY updated_at, user_id) AS n
    FROM identity.user_entitlements
    WHERE tier <> 'free'
)
UPDATE identity.user_entitlements e
SET member_number = ordered.n
FROM ordered
WHERE e.user_id = ordered.user_id;

SELECT setval('identity.pro_member_number_seq', COALESCE((SELECT max(member_number) FROM identity.user_entitlements), 0) + 1, false);
