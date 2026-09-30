DROP TABLE IF EXISTS identity.trial_grants;

DROP INDEX IF EXISTS identity.user_entitlements_expiring_idx;

UPDATE identity.user_entitlements SET source = 'manual' WHERE source IN ('trial', 'expiry');

ALTER TABLE identity.user_entitlements
    DROP CONSTRAINT user_entitlements_source_check,
    ADD CONSTRAINT user_entitlements_source_check CHECK (source IN ('manual', 'billing')),
    DROP COLUMN trial_used_at;
