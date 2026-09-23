CREATE TABLE realtime.realtime_presence_preferences (
    user_id uuid NOT NULL,
    appear_offline boolean DEFAULT false NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT realtime_presence_preferences_pkey PRIMARY KEY (user_id)
);
