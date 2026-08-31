CREATE TABLE voice.call_invite_outcomes (
    call_id     UUID NOT NULL,
    user_id     UUID NOT NULL,
    inviter_id  UUID NOT NULL,
    answered_at TIMESTAMPTZ,
    declined    BOOLEAN NOT NULL DEFAULT false,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_call_invite_outcomes_lookup
    ON voice.call_invite_outcomes (call_id, user_id, inviter_id, created_at DESC);
