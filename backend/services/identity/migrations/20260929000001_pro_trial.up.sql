ALTER TABLE identity.user_entitlements
    ADD COLUMN trial_used_at timestamp with time zone,
    DROP CONSTRAINT user_entitlements_source_check,
    ADD CONSTRAINT user_entitlements_source_check CHECK (source IN ('manual', 'billing', 'trial', 'expiry'));

CREATE INDEX user_entitlements_expiring_idx
    ON identity.user_entitlements (valid_until)
    WHERE tier <> 'free' AND valid_until IS NOT NULL;

CREATE TABLE identity.trial_grants (
    id bigserial NOT NULL,
    user_id uuid,
    email_hash character(64) NOT NULL,
    ip_hash character(64),
    granted_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT trial_grants_pkey PRIMARY KEY (id),
    CONSTRAINT trial_grants_user_id_fkey FOREIGN KEY (user_id) REFERENCES identity.users(id) ON DELETE SET NULL
);

CREATE UNIQUE INDEX trial_grants_email_hash_idx ON identity.trial_grants (email_hash);
CREATE INDEX trial_grants_ip_hash_idx ON identity.trial_grants (ip_hash, granted_at) WHERE ip_hash IS NOT NULL;
