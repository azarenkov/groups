CREATE TYPE verification_purpose AS ENUM ('registration', 'password_reset', 'email_change', 'phone_change');
CREATE TYPE verification_channel AS ENUM ('email', 'phone');

CREATE TABLE verification_codes (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    channel     verification_channel NOT NULL,
    code_hash   VARCHAR(255) NOT NULL,
    purpose     verification_purpose NOT NULL,
    expires_at  TIMESTAMPTZ NOT NULL,
    consumed_at TIMESTAMPTZ,
    attempts    SMALLINT NOT NULL DEFAULT 0,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),

    CONSTRAINT verification_codes_attempts_check
        CHECK (attempts >= 0 AND attempts <= 10),
    CONSTRAINT verification_codes_expires_after_created_check
        CHECK (expires_at > created_at)
);

CREATE INDEX verification_codes_lookup_idx
    ON verification_codes (user_id, channel, purpose, created_at DESC);

CREATE INDEX verification_codes_expires_idx
    ON verification_codes (expires_at)
    WHERE consumed_at IS NULL;
