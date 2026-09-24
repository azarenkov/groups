CREATE TABLE messaging.hidden_messages (
    user_id uuid NOT NULL,
    message_id uuid NOT NULL,
    hidden_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, message_id)
);
