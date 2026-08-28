ALTER TABLE identity.users
    ADD COLUMN pending_email       VARCHAR(255),
    ADD COLUMN password_changed_at TIMESTAMPTZ,
    ADD COLUMN disabled_at         TIMESTAMPTZ,
    ADD COLUMN deleted_at          TIMESTAMPTZ;
